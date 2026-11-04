use crate::drivers::hal::now_ms;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyConfig {
    pub index: u8,
    pub title: String,
    pub subtitle: String,
    pub icon_name: String,
    pub action_id: String,
    pub action_label: String,
    pub action_category: String,
    pub action_param: String,
    pub accent_color: String,
    pub bg_color: String,
    pub show_title: bool,
    pub title_size: String,
    pub background_style: String,
    pub corner_radius: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchZoneConfig {
    pub index: u8,
    pub label: String,
    pub value_text: String,
    pub tap_action_id: String,
    pub accent_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncoderConfig {
    pub index: u8,
    pub label: String,
    pub value_text: String,
    pub cw_action: String,
    pub ccw_action: String,
    pub push_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePage {
    pub id: String,
    pub name: String,
    pub keys: Vec<KeyConfig>,
    pub touch_zones: Vec<TouchZoneConfig>,
    pub encoders: Vec<EncoderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoSwitchRule {
    pub id: String,
    pub process_name: String,
    pub window_title_contains: String,
    pub target_profile_id: String,
    pub priority: u8,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color_strip: String,
    pub target_device_id: String,
    pub pages: Vec<ProfilePage>,
    pub last_modified_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroStep {
    pub id: String,
    pub step_type: String,
    pub label: String,
    pub parameter: String,
    pub timeout_ms: u32,
    pub on_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroSequence {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<MacroStep>,
    pub last_edited_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPackageInfo {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub source_kind: String,
    pub installed: bool,
    pub enabled: bool,
    pub permissions: Vec<String>,
    pub actions_provided: Vec<String>,
    pub disk_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyStreamDockSummary {
    pub profiles_dir: String,
    pub plugins_dir: String,
    pub legacy_profile_count: usize,
    pub named_root_profiles_count: usize,
    pub legacy_plugin_count: usize,
    pub discovered_plugins: Vec<PluginPackageInfo>,
    pub plugin_ws_port: u16,
    pub plugin_host_active: bool,
}

pub fn default_keys() -> Vec<KeyConfig> {
    let presets = [
        ("Start Stream", "LIVE", "Radio", "obs.start_stream", "OBS: Start Streaming", "OBS", "Main Scene", "#F16B79", "#2A1521"),
        ("Scene +", "NEXT", "Clapperboard", "obs.next_scene", "OBS: Next Scene", "OBS", "Scene Collection", "#56B9F2", "#132235"),
        ("Scene -", "PREV", "Film", "obs.prev_scene", "OBS: Previous Scene", "OBS", "Scene Collection", "#56B9F2", "#132235"),
        ("Mute Mic", "MUTE", "MicOff", "audio.mute_mic", "Audio: Toggle Mic Mute", "Media", "VK_VOLUME_MUTE", "#F16B79", "#2A1521"),
        ("Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Master Volume Up", "Media", "VK_VOLUME_UP", "#35C98B", "#112822"),
        ("Record", "REC", "CircleDot", "obs.toggle_record", "OBS: Toggle Recording", "OBS", "MKV High", "#F16B79", "#2A1521"),
        ("Clip 60s", "CLIP", "Scissors", "obs.save_replay", "OBS: Save Replay Buffer", "Streaming", "Last 60s", "#8B5CF6", "#231B38"),
        ("Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Capture Screen (Win+Shift+S)", "System", "Win+Shift+S", "#56B9F2", "#132235"),
        ("Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager (Ctrl+Shift+Esc)", "System", "Ctrl+Shift+Esc", "#8B5CF6", "#231B38"),
        ("Show Desktop", "DESK", "Monitor", "sys.show_desktop", "System: Show Desktop (Win+D)", "System", "Win+D", "#56B9F2", "#132235"),
        ("Explorer", "FILE", "Folder", "sys.hotkey", "System: File Explorer (Win+E)", "System", "Win+E", "#F4B860", "#2A2215"),
        ("Audio Mixer", "MIX", "Sliders", "audio.open_mixer", "Audio: Open Volume Mixer (sndvol.exe)", "Media", "sndvol.exe", "#8B5CF6", "#231B38"),
        ("Spotify", "PLAY", "Music", "media.play_pause", "Media: Play / Pause Track", "Media", "VK_MEDIA_PLAY_PAUSE", "#35C98B", "#112822"),
        ("Settings", "WIN+I", "Settings", "sys.hotkey", "System: Windows Settings (Win+I)", "System", "Win+I", "#56B9F2", "#132235"),
        ("Diagnostics", "SYS", "LayoutGrid", "profile.switch_page", "Navigation: Open Diagnostics Page", "System", "Page 2", "#8B5CF6", "#231B38"),
    ];

    let mut out = Vec::with_capacity(32);
    for i in 0..32u8 {
        let p = &presets[(i as usize) % presets.len()];
        out.push(KeyConfig {
            index: i,
            title: p.0.to_string(),
            subtitle: p.1.to_string(),
            icon_name: p.2.to_string(),
            action_id: p.3.to_string(),
            action_label: p.4.to_string(),
            action_category: p.5.to_string(),
            action_param: p.6.to_string(),
            accent_color: p.7.to_string(),
            bg_color: p.8.to_string(),
            show_title: true,
            title_size: "Medium".to_string(),
            background_style: "Gradient".to_string(),
            corner_radius: 8,
        });
    }
    out
}

pub fn default_touch_zones() -> Vec<TouchZoneConfig> {
    vec![
        TouchZoneConfig {
            index: 0,
            label: "SPOTIFY".to_string(),
            value_text: "▶ Play / Pause".to_string(),
            tap_action_id: "media.play_pause".to_string(),
            accent_color: "#35C98B".to_string(),
        },
        TouchZoneConfig {
            index: 1,
            label: "CPU LOAD".to_string(),
            value_text: "Live Telemetry".to_string(),
            tap_action_id: "sys.hotkey".to_string(),
            accent_color: "#56B9F2".to_string(),
        },
        TouchZoneConfig {
            index: 2,
            label: "MIXER".to_string(),
            value_text: "Open Audio".to_string(),
            tap_action_id: "audio.open_mixer".to_string(),
            accent_color: "#8B5CF6".to_string(),
        },
        TouchZoneConfig {
            index: 3,
            label: "MIC MUTE".to_string(),
            value_text: "Toggle Mute".to_string(),
            tap_action_id: "audio.mute_mic".to_string(),
            accent_color: "#F4B860".to_string(),
        },
    ]
}

pub fn default_encoders() -> Vec<EncoderConfig> {
    vec![
        EncoderConfig {
            index: 0,
            label: "Master Vol".to_string(),
            value_text: "Win32 Vol ±".to_string(),
            cw_action: "audio.volume_up".to_string(),
            ccw_action: "audio.volume_down".to_string(),
            push_action: "audio.volume_mute".to_string(),
        },
        EncoderConfig {
            index: 1,
            label: "Media Track".to_string(),
            value_text: "Prev / Next".to_string(),
            cw_action: "media.next".to_string(),
            ccw_action: "media.prev".to_string(),
            push_action: "media.play_pause".to_string(),
        },
        EncoderConfig {
            index: 2,
            label: "Timeline".to_string(),
            value_text: "Left / Right".to_string(),
            cw_action: "Right".to_string(),
            ccw_action: "Left".to_string(),
            push_action: "Space".to_string(),
        },
        EncoderConfig {
            index: 3,
            label: "Brightness".to_string(),
            value_text: "85% • CRT..LIG".to_string(),
            cw_action: "brightness.up".to_string(),
            ccw_action: "brightness.down".to_string(),
            push_action: "brightness.toggle".to_string(),
        },
    ]
}

pub fn default_profiles() -> Vec<Profile> {
    let ts = now_ms();
    let page1 = ProfilePage {
        id: "page-1".to_string(),
        name: "Page 1".to_string(),
        keys: default_keys(),
        touch_zones: default_touch_zones(),
        encoders: default_encoders(),
    };
    let mut page2_keys = default_keys();
    page2_keys[0].title = "CPU Monitor".to_string();
    page2_keys[0].action_label = "System: Live CPU Telemetry".to_string();
    page2_keys[1].title = "RAM Usage".to_string();
    page2_keys[1].action_label = "System: Live RAM Telemetry".to_string();
    let page2 = ProfilePage {
        id: "page-2".to_string(),
        name: "Page 2 (Telemetry)".to_string(),
        keys: page2_keys,
        touch_zones: default_touch_zones(),
        encoders: default_encoders(),
    };

    vec![
        Profile {
            id: "profile-streaming".to_string(),
            name: "Streaming".to_string(),
            icon: "Radio".to_string(),
            color_strip: "#8B5CF6".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![page1.clone(), page2],
            last_modified_ms: ts,
        },
        Profile {
            id: "profile-gaming".to_string(),
            name: "Gaming".to_string(),
            icon: "Gamepad2".to_string(),
            color_strip: "#35C98B".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![page1.clone()],
            last_modified_ms: ts,
        },
        Profile {
            id: "profile-editing".to_string(),
            name: "Editing".to_string(),
            icon: "Film".to_string(),
            color_strip: "#56B9F2".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![page1.clone()],
            last_modified_ms: ts,
        },
        Profile {
            id: "profile-work".to_string(),
            name: "Work".to_string(),
            icon: "Briefcase".to_string(),
            color_strip: "#F4B860".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![page1.clone()],
            last_modified_ms: ts,
        },
        Profile {
            id: "profile-default".to_string(),
            name: "Default".to_string(),
            icon: "Layers".to_string(),
            color_strip: "#A6AFC4".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![page1],
            last_modified_ms: ts,
        },
    ]
}

pub fn default_auto_switch_rules() -> Vec<AutoSwitchRule> {
    vec![
        AutoSwitchRule {
            id: "rule-obs".to_string(),
            process_name: "obs64.exe".to_string(),
            window_title_contains: "OBS".to_string(),
            target_profile_id: "profile-streaming".to_string(),
            priority: 1,
            enabled: true,
        },
        AutoSwitchRule {
            id: "rule-resolve".to_string(),
            process_name: "Resolve.exe".to_string(),
            window_title_contains: "DaVinci Resolve".to_string(),
            target_profile_id: "profile-editing".to_string(),
            priority: 2,
            enabled: true,
        },
        AutoSwitchRule {
            id: "rule-code".to_string(),
            process_name: "Code.exe".to_string(),
            window_title_contains: "Visual Studio Code".to_string(),
            target_profile_id: "profile-work".to_string(),
            priority: 3,
            enabled: true,
        },
    ]
}

pub fn default_macros() -> Vec<MacroSequence> {
    let ts = now_ms();
    vec![
        MacroSequence {
            id: "macro-go-live".to_string(),
            name: "Go Live Broadcast Prep".to_string(),
            description: "Checks OBS Studio, dispatches Starting Soon hotkey, waits 500ms, and switches to Streaming profile.".to_string(),
            steps: vec![
                MacroStep {
                    id: "step-1".to_string(),
                    step_type: "IfWindowExists".to_string(),
                    label: "Check if OBS Studio (obs64.exe) is running".to_string(),
                    parameter: "obs64.exe".to_string(),
                    timeout_ms: 2000,
                    on_error: "Continue".to_string(),
                },
                MacroStep {
                    id: "step-2".to_string(),
                    step_type: "Shortcut".to_string(),
                    label: "Send OBS Starting Soon Scene Hotkey".to_string(),
                    parameter: "Ctrl+Alt+F1".to_string(),
                    timeout_ms: 500,
                    on_error: "Stop".to_string(),
                },
                MacroStep {
                    id: "step-3".to_string(),
                    step_type: "Delay".to_string(),
                    label: "Wait 500ms for scene transition".to_string(),
                    parameter: "500".to_string(),
                    timeout_ms: 1000,
                    on_error: "Continue".to_string(),
                },
                MacroStep {
                    id: "step-4".to_string(),
                    step_type: "SwitchProfile".to_string(),
                    label: "Switch DeckForge Profile → Streaming".to_string(),
                    parameter: "profile-streaming".to_string(),
                    timeout_ms: 500,
                    on_error: "Stop".to_string(),
                },
            ],
            last_edited_ms: ts,
        },
        MacroSequence {
            id: "macro-dev-workspace".to_string(),
            name: "Developer Focus Setup".to_string(),
            description: "Launches Windows Terminal, waits 300ms, and snaps window left.".to_string(),
            steps: vec![
                MacroStep {
                    id: "step-201".to_string(),
                    step_type: "LaunchApp".to_string(),
                    label: "Launch Windows Terminal".to_string(),
                    parameter: "wt.exe".to_string(),
                    timeout_ms: 3000,
                    on_error: "Continue".to_string(),
                },
                MacroStep {
                    id: "step-202".to_string(),
                    step_type: "Delay".to_string(),
                    label: "Wait 300ms".to_string(),
                    parameter: "300".to_string(),
                    timeout_ms: 500,
                    on_error: "Continue".to_string(),
                },
                MacroStep {
                    id: "step-203".to_string(),
                    step_type: "Shortcut".to_string(),
                    label: "Snap Window Left (Win+Left)".to_string(),
                    parameter: "Win+Left".to_string(),
                    timeout_ms: 500,
                    on_error: "Continue".to_string(),
                },
            ],
            last_edited_ms: ts,
        },
    ]
}

/// Converts a Windows Virtual-Key integer (`VKeyCode` from `.sdProfile` JSON) into its key name.
fn vk_code_to_name(vk: i64) -> Option<String> {
    match vk {
        27 => Some("Esc".to_string()),
        32 => Some("Space".to_string()),
        37 => Some("Left".to_string()),
        38 => Some("Up".to_string()),
        39 => Some("Right".to_string()),
        40 => Some("Down".to_string()),
        48..=57 => Some(char::from_u32(vk as u32)?.to_string()),
        65..=90 => Some(char::from_u32(vk as u32)?.to_string()),
        107 => Some("Plus".to_string()),
        109 => Some("Minus".to_string()),
        112..=123 => Some(format!("F{}", vk - 111)),
        _ => None,
    }
}

/// Parses a single `.sdProfile` action JSON object into a `KeyConfig`.
fn parse_sdprofile_action(key_index: u8, act_val: &serde_json::Value) -> KeyConfig {
    let raw_name = act_val
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("Action");
    let uuid = act_val
        .get("UUID")
        .and_then(|v| v.as_str())
        .unwrap_or("custom.action");

    // Extract hotkey chord if present in Settings.Hotkeys[0]
    let mut chord = String::new();
    if let Some(hotkeys) = act_val
        .get("Settings")
        .and_then(|s| s.get("Hotkeys"))
        .and_then(|h| h.as_array())
    {
        if let Some(first) = hotkeys.first() {
            let vkey = first.get("VKeyCode").and_then(|v| v.as_i64()).unwrap_or(-1);
            if let Some(kname) = vk_code_to_name(vkey) {
                let mut parts = Vec::new();
                if first.get("KeyCmd").and_then(|v| v.as_bool()).unwrap_or(false) {
                    parts.push("Win".to_string());
                }
                if first.get("KeyCtrl").and_then(|v| v.as_bool()).unwrap_or(false) {
                    parts.push("Ctrl".to_string());
                }
                if first.get("KeyOption").and_then(|v| v.as_bool()).unwrap_or(false) {
                    parts.push("Alt".to_string());
                }
                if first.get("KeyShift").and_then(|v| v.as_bool()).unwrap_or(false) {
                    parts.push("Shift".to_string());
                }
                parts.push(kname);
                chord = parts.join("+");
            }
        }
    }

    let (title, icon_name, action_id, action_label, param, accent, bg) = if !chord.is_empty() {
        let friendly = match chord.as_str() {
            "Win+D" => "Show Desktop",
            "Win+E" => "File Explorer",
            "Win+I" => "Win Settings",
            "Win+L" => "Lock PC",
            "Win+R" => "Run Dialog",
            "Win+X" => "Power Menu",
            "Win+A" => "Quick Settings",
            "Ctrl+Shift+Esc" => "Task Manager",
            "Alt+F4" => "Close Window",
            _ => &chord,
        };
        (
            friendly.to_string(),
            "Zap".to_string(),
            "sys.hotkey".to_string(),
            format!("Hotkey: {}", chord),
            chord.clone(),
            "#8B5CF6".to_string(),
            "#231B38".to_string(),
        )
    } else if uuid.contains("volume") {
        (
            "System Vol".to_string(),
            "Volume2".to_string(),
            "audio.volume_up".to_string(),
            "Audio: System Volume".to_string(),
            "VK_VOLUME_UP".to_string(),
            "#35C98B".to_string(),
            "#112822".to_string(),
        )
    } else if uuid.contains("page.change") || uuid.contains("profile.rotate") {
        (
            raw_name.to_string(),
            "Layers".to_string(),
            "profile.switch_page".to_string(),
            format!("Navigation: {}", raw_name),
            "Next Page".to_string(),
            "#56B9F2".to_string(),
            "#132235".to_string(),
        )
    } else {
        let clean_title = if raw_name.contains('?') || raw_name.trim().is_empty() {
            uuid.split('.').next_back().unwrap_or("Plugin")
        } else {
            raw_name
        };
        (
            clean_title.to_string(),
            "Puzzle".to_string(),
            "plugin.sd_bridge".to_string(),
            format!("Plugin: {}", uuid),
            uuid.to_string(),
            "#F4B860".to_string(),
            "#2A2215".to_string(),
        )
    };

    KeyConfig {
        index: key_index,
        title,
        subtitle: "IMPORTED".to_string(),
        icon_name,
        action_id,
        action_label,
        action_category: "Imported".to_string(),
        action_param: param,
        accent_color: accent,
        bg_color: bg,
        show_title: true,
        title_size: "Medium".to_string(),
        background_style: "Gradient".to_string(),
        corner_radius: 8,
    }
}

/// Parses all `.sdProfile` folders in `%APPDATA%\HotSpot\StreamDock\profiles` and returns
/// rich `Profile` structs ready to use in DeckForge!
pub fn parse_all_legacy_sdprofiles() -> Vec<Profile> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\MAHMOOD\\AppData\\Roaming".to_string());
    let profiles_dir = PathBuf::from(&appdata).join("HotSpot").join("StreamDock").join("profiles");

    let mut imported = Vec::new();
    let Ok(entries) = fs::read_dir(&profiles_dir) else {
        return imported;
    };

    for entry in entries.flatten() {
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }
        let manifest_path = dir_path.join("manifest.json");
        let Ok(raw) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };

        let raw_name = json
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let dev_uuid = json
            .get("DeviceUUID")
            .and_then(|v| v.as_str())
            .unwrap_or("StreamDock");
        let actions_obj = json.get("Actions").and_then(|v| v.as_object());
        let action_count = actions_obj.map(|a| a.len()).unwrap_or(0);

        // Only import top-level named profiles or profiles with >= 3 configured actions
        if raw_name.is_empty() && action_count < 3 {
            continue;
        }

        let folder_id = dir_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("sdprofile-{}", now_ms()));
        let short_id = folder_id.chars().take(8).collect::<String>();

        let display_name = if !raw_name.is_empty() {
            format!("{} ({})", raw_name, dev_uuid)
        } else {
            format!("Legacy {} [{}]", dev_uuid, short_id)
        };

        // Start with clean empty keys and overlay the parsed actions
        let mut keys: Vec<KeyConfig> = (0..32u8)
            .map(|i| KeyConfig {
                index: i,
                title: "Empty".to_string(),
                subtitle: "".to_string(),
                icon_name: "Plus".to_string(),
                action_id: "none".to_string(),
                action_label: "No Action Assigned".to_string(),
                action_category: "System".to_string(),
                action_param: "".to_string(),
                accent_color: "#394158".to_string(),
                bg_color: "#151925".to_string(),
                show_title: true,
                title_size: "Medium".to_string(),
                background_style: "Gradient".to_string(),
                corner_radius: 8,
            })
            .collect();

        if let Some(actions_map) = actions_obj {
            for (coord, act_val) in actions_map {
                let parts: Vec<&str> = coord.split(',').collect();
                if parts.len() == 2 {
                    let col = parts[0].parse::<u8>().unwrap_or(0);
                    let row = parts[1].parse::<u8>().unwrap_or(0);
                    // Map (col, row) onto 5-column grid (rows 0..1 = LCD keys 0..9; row 2 = slots 10..14)
                    let idx = (row.saturating_mul(5).saturating_add(col)) as usize;
                    if idx < keys.len() {
                        keys[idx] = parse_sdprofile_action(idx as u8, act_val);
                    }
                }
            }
        }

        imported.push(Profile {
            id: format!("imported-{}", folder_id),
            name: display_name,
            icon: "Sparkles".to_string(),
            color_strip: "#35C98B".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![ProfilePage {
                id: "page-1".to_string(),
                name: "Imported Page 1".to_string(),
                keys,
                touch_zones: default_touch_zones(),
                encoders: default_encoders(),
            }],
            last_modified_ms: now_ms(),
        });
    }

    imported
}

/// Scans `%APPDATA%\HotSpot\StreamDock\profiles` and `%APPDATA%\HotSpot\StreamDock\plugins`.
pub fn scan_legacy_streamdock_environment() -> LegacyStreamDockSummary {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\MAHMOOD\\AppData\\Roaming".to_string());
    let base = PathBuf::from(&appdata).join("HotSpot").join("StreamDock");
    let profiles_dir = base.join("profiles");
    let plugins_dir = base.join("plugins");

    let legacy_profile_count = fs::read_dir(&profiles_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .count()
        })
        .unwrap_or(0);

    let named_root_profiles_count = parse_all_legacy_sdprofiles().len();

    let mut discovered_plugins = vec![
        PluginPackageInfo {
            id: "builtin.obs".to_string(),
            name: "OBS Studio WebSocket".to_string(),
            author: "DeckForge Core".to_string(),
            version: "1.0.0".to_string(),
            description: "Native scene switching, stream/record control, replay buffer, and audio source mute.".to_string(),
            category: "Streaming".to_string(),
            source_kind: "BuiltIn".to_string(),
            installed: true,
            enabled: true,
            permissions: vec!["Local Network (ws://127.0.0.1:4455)".to_string()],
            actions_provided: vec!["Start/Stop Stream".to_string(), "Switch Scene".to_string(), "Save Replay".to_string()],
            disk_path: None,
        },
        PluginPackageInfo {
            id: "builtin.audio".to_string(),
            name: "Windows Audio & Media Mixer".to_string(),
            author: "DeckForge Core".to_string(),
            version: "1.0.0".to_string(),
            description: "Real Win32 volume/media Virtual-Key control (`VK_VOLUME_UP/DOWN/MUTE`, `VK_MEDIA_PLAY_PAUSE`) and rotary encoder binding.".to_string(),
            category: "Media".to_string(),
            source_kind: "BuiltIn".to_string(),
            installed: true,
            enabled: true,
            permissions: vec!["Win32 Audio & Media Keys".to_string()],
            actions_provided: vec!["Toggle Mic Mute".to_string(), "Master Volume ±".to_string(), "Media Play/Pause".to_string()],
            disk_path: None,
        },
        PluginPackageInfo {
            id: "builtin.system".to_string(),
            name: "System & Win32 Hotkey Engine".to_string(),
            author: "DeckForge Core".to_string(),
            version: "1.0.0".to_string(),
            description: "Direct `user32.dll` `keybd_event` chord injection (`Win+D`, `Win+E`, `Ctrl+Shift+Esc`, `Alt+F4`) & app launcher.".to_string(),
            category: "System".to_string(),
            source_kind: "BuiltIn".to_string(),
            installed: true,
            enabled: true,
            permissions: vec!["Win32 Keyboard Input".to_string(), "Process Launch".to_string()],
            actions_provided: vec!["Send Hotkey Chord".to_string(), "Launch App / URL".to_string(), "Show Desktop".to_string()],
            disk_path: None,
        },
    ];

    let mut legacy_plugin_count = 0usize;
    if let Ok(entries) = fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                legacy_plugin_count += 1;
                let folder_name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let manifest_path = p.join("manifest.json");
                let mut name = folder_name.trim_end_matches(".sdPlugin").to_string();
                let mut author = "StreamDock Plugin".to_string();
                let mut version = "1.0.0".to_string();
                let mut description = format!("Discovered local .sdPlugin package ({})", folder_name);
                let mut actions_provided = Vec::new();

                if let Ok(raw) = fs::read_to_string(&manifest_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                        if let Some(n) = val.get("Name").and_then(|v| v.as_str()) {
                            name = n.to_string();
                        }
                        if let Some(a) = val.get("Author").and_then(|v| v.as_str()) {
                            author = a.to_string();
                        }
                        if let Some(v) = val.get("Version").and_then(|v| v.as_str()) {
                            version = v.to_string();
                        }
                        if let Some(d) = val.get("Description").and_then(|v| v.as_str()) {
                            description = d.to_string();
                        }
                        if let Some(acts) = val.get("Actions").and_then(|v| v.as_array()) {
                            for act in acts {
                                if let Some(an) = act.get("Name").and_then(|v| v.as_str()) {
                                    actions_provided.push(an.to_string());
                                }
                            }
                        }
                    }
                }

                discovered_plugins.push(PluginPackageInfo {
                    id: folder_name.clone(),
                    name,
                    author,
                    version,
                    description,
                    category: "Plugins".to_string(),
                    source_kind: "LocalSdPlugin".to_string(),
                    installed: true,
                    enabled: !folder_name.contains("coding-subscribe-usage"),
                    permissions: vec![
                        "Local WebSocket Bridge (ws://127.0.0.1:28196)".to_string(),
                        "Sandboxed Worker Process".to_string(),
                    ],
                    actions_provided,
                    disk_path: Some(p.to_string_lossy().to_string()),
                });
            }
        }
    }

    LegacyStreamDockSummary {
        profiles_dir: profiles_dir.to_string_lossy().to_string(),
        plugins_dir: plugins_dir.to_string_lossy().to_string(),
        legacy_profile_count,
        named_root_profiles_count,
        legacy_plugin_count,
        discovered_plugins,
        plugin_ws_port: 28196,
        plugin_host_active: true,
    }
}
