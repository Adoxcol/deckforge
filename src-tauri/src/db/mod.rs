use crate::drivers::hal::now_ms;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenMediaConfig {
    pub media_id: String,
    pub fit: String,
    pub loop_playback: bool,
    pub speed: f32,
}

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
    #[serde(default)]
    pub media: Option<ScreenMediaConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchZoneConfig {
    pub index: u8,
    pub label: String,
    pub value_text: String,
    pub tap_action_id: String,
    pub accent_color: String,
    #[serde(default)]
    pub tap_action_param: String,
    #[serde(default)]
    pub media: Option<ScreenMediaConfig>,
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
    #[serde(default)]
    pub cw_param: String,
    #[serde(default)]
    pub ccw_param: String,
    #[serde(default)]
    pub push_param: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePage {
    pub id: String,
    pub name: String,
    pub keys: Vec<KeyConfig>,
    pub touch_zones: Vec<TouchZoneConfig>,
    pub encoders: Vec<EncoderConfig>,
    #[serde(default)]
    pub strip_media: Option<ScreenMediaConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoSwitchRule {
    pub id: String,
    pub process_name: String,
    pub window_title_contains: String,
    pub target_profile_id: String,
    /// Page to open in that profile. Empty keeps the old behaviour: page 1, and no switch
    /// when that profile is already showing.
    #[serde(default)]
    pub target_page_id: String,
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
    pub plugin_ws_port: Option<u16>,
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
        ("Desktop", "PAGE", "Monitor", "profile.switch_page", "Navigation: Open Desktop Page", "Navigation", "Desktop", "#56B9F2", "#132235"),
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
            media: None,
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
            tap_action_param: String::new(),
            media: None,
        },
        TouchZoneConfig {
            index: 1,
            label: "NEXT PAGE".to_string(),
            value_text: "Next page".to_string(),
            tap_action_id: "profile.next_page".to_string(),
            accent_color: "#56B9F2".to_string(),
            tap_action_param: String::new(),
            media: None,
        },
        TouchZoneConfig {
            index: 2,
            label: "MIXER".to_string(),
            value_text: "Open Audio".to_string(),
            tap_action_id: "audio.open_mixer".to_string(),
            accent_color: "#8B5CF6".to_string(),
            tap_action_param: String::new(),
            media: None,
        },
        TouchZoneConfig {
            index: 3,
            label: "MIC MUTE".to_string(),
            value_text: "Toggle Mute".to_string(),
            tap_action_id: "audio.mute_mic".to_string(),
            accent_color: "#F4B860".to_string(),
            tap_action_param: String::new(),
            media: None,
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
            cw_param: String::new(),
            ccw_param: String::new(),
            push_param: String::new(),
        },
        EncoderConfig {
            index: 1,
            label: "Media Track".to_string(),
            value_text: "Prev / Next".to_string(),
            cw_action: "media.next".to_string(),
            ccw_action: "media.prev".to_string(),
            push_action: "media.play_pause".to_string(),
            cw_param: String::new(),
            ccw_param: String::new(),
            push_param: String::new(),
        },
        EncoderConfig {
            index: 2,
            label: "Timeline".to_string(),
            value_text: "Left / Right".to_string(),
            cw_action: "Right".to_string(),
            ccw_action: "Left".to_string(),
            push_action: "Space".to_string(),
            cw_param: String::new(),
            ccw_param: String::new(),
            push_param: String::new(),
        },
        EncoderConfig {
            index: 3,
            label: "Brightness".to_string(),
            value_text: "85% • CRT..LIG".to_string(),
            cw_action: "brightness.up".to_string(),
            ccw_action: "brightness.down".to_string(),
            push_action: "brightness.toggle".to_string(),
            cw_param: String::new(),
            ccw_param: String::new(),
            push_param: String::new(),
        },
    ]
}

type KeySpec = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

fn paint_keys(specs: &[KeySpec], accent: &str, bg: &str) -> Vec<KeyConfig> {
    let mut out = Vec::with_capacity(32);
    for i in 0..32u8 {
        let spec = specs[(i as usize) % specs.len()];
        out.push(KeyConfig {
            index: i,
            title: spec.0.to_string(),
            subtitle: spec.1.to_string(),
            icon_name: spec.2.to_string(),
            action_id: spec.3.to_string(),
            action_label: spec.4.to_string(),
            action_category: spec.5.to_string(),
            action_param: spec.6.to_string(),
            accent_color: accent.to_string(),
            bg_color: bg.to_string(),
            show_title: true,
            title_size: "Medium".to_string(),
            background_style: "Gradient".to_string(),
            corner_radius: 8,
            media: None,
        });
    }
    out
}

fn zone(index: u8, label: &str, value: &str, action: &str, color: &str) -> TouchZoneConfig {
    TouchZoneConfig {
        index,
        label: label.to_string(),
        value_text: value.to_string(),
        tap_action_id: action.to_string(),
        accent_color: color.to_string(),
        tap_action_param: String::new(),
        media: None,
    }
}

fn nav_zones(label: &str, value: &str, action: &str, color: &str) -> Vec<TouchZoneConfig> {
    vec![
        zone(0, label, value, action, color),
        zone(1, "PREV PAGE", "Previous page", "profile.prev_page", "#56B9F2"),
        zone(2, "PAGE", "Swipe to change", "profile.next_page", "#8B5CF6"),
        zone(3, "NEXT PAGE", "Next page", "profile.next_page", "#F4B860"),
    ]
}

fn make_page(id: &str, name: &str, keys: Vec<KeyConfig>, zones: Vec<TouchZoneConfig>) -> ProfilePage {
    ProfilePage {
        id: id.to_string(),
        name: name.to_string(),
        keys,
        touch_zones: zones,
        encoders: default_encoders(),
        strip_media: None,
    }
}

fn music_page() -> ProfilePage {
    make_page(
        "page-music",
        "Music",
        paint_keys(
            &[
                ("Play", "SPOTIFY", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""),
                ("Next", "TRACK", "Music", "media.next", "Media: Next Track", "Media", ""),
                ("Previous", "TRACK", "Disc", "media.prev", "Media: Previous Track", "Media", ""),
                ("Stop", "MEDIA", "Disc", "media.stop", "Media: Stop", "Media", ""),
                ("Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""),
                ("Vol Down", "-2%", "Volume2", "audio.volume_down", "Audio: Volume Down", "Media", ""),
                ("Mute", "SOUND", "Volume2", "audio.volume_mute", "Audio: Mute Speakers", "Media", ""),
                ("Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""),
                ("Mixer", "AUDIO", "Volume2", "audio.open_mixer", "Audio: Open Mixer", "Media", ""),
                ("Stream", "PAGE", "Radio", "profile.switch_page", "Navigation: Stream Page", "Navigation", "Page 1"),
            ],
            "#35C98B",
            "#112822",
        ),
        nav_zones("SPOTIFY", "Play / Pause", "media.play_pause", "#35C98B"),
    )
}

fn desktop_page() -> ProfilePage {
    make_page(
        "page-desktop",
        "Desktop",
        paint_keys(
            &[
                ("Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"),
                ("Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"),
                ("Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager", "System", "Ctrl+Shift+Esc"),
                ("Files", "OPEN", "Folder", "sys.hotkey", "System: File Explorer", "System", "Win+E"),
                ("Lock", "PC", "Zap", "sys.lock_pc", "System: Lock PC", "System", ""),
                ("Settings", "WIN", "Settings", "sys.hotkey", "System: Windows Settings", "System", "Win+I"),
                ("Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""),
                ("Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""),
                ("Brighter", "DECK", "Zap", "brightness.up", "Deck: Brightness Up", "System", ""),
                ("Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"),
            ],
            "#56B9F2",
            "#132235",
        ),
        nav_zones("SNAP", "Screenshot", "sys.screenshot", "#56B9F2"),
    )
}

fn game_page() -> ProfilePage {
    make_page(
        "page-game",
        "Game",
        paint_keys(
            &[
                ("Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""),
                ("Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""),
                ("Vol Down", "-2%", "Volume2", "audio.volume_down", "Audio: Volume Down", "Media", ""),
                ("Mute", "SOUND", "Volume2", "audio.volume_mute", "Audio: Mute Speakers", "Media", ""),
                ("Play", "MUSIC", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""),
                ("Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"),
                ("Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"),
                ("Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager", "System", "Ctrl+Shift+Esc"),
                ("Lock", "PC", "Zap", "sys.lock_pc", "System: Lock PC", "System", ""),
                ("Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"),
            ],
            "#35C98B",
            "#10241C",
        ),
        nav_zones("MIC", "Mute mic", "audio.mute_mic", "#35C98B"),
    )
}

fn edit_page() -> ProfilePage {
    make_page(
        "page-edit",
        "Edit",
        paint_keys(
            &[
                ("Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"),
                ("Play", "REVIEW", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""),
                ("Previous", "CLIP", "Disc", "media.prev", "Media: Previous Track", "Media", ""),
                ("Next", "CLIP", "Music", "media.next", "Media: Next Track", "Media", ""),
                ("Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""),
                ("Vol Down", "-2%", "Volume2", "audio.volume_down", "Audio: Volume Down", "Media", ""),
                ("Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""),
                ("Files", "OPEN", "Folder", "sys.hotkey", "System: File Explorer", "System", "Win+E"),
                ("Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"),
                ("Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"),
            ],
            "#56B9F2",
            "#10202C",
        ),
        nav_zones("SNAP", "Screenshot", "sys.screenshot", "#56B9F2"),
    )
}

fn work_page() -> ProfilePage {
    make_page(
        "page-work",
        "Work",
        paint_keys(
            &[
                ("Files", "OPEN", "Folder", "sys.hotkey", "System: File Explorer", "System", "Win+E"),
                ("Settings", "WIN", "Settings", "sys.hotkey", "System: Windows Settings", "System", "Win+I"),
                ("Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"),
                ("Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"),
                ("Lock", "PC", "Zap", "sys.lock_pc", "System: Lock PC", "System", ""),
                ("Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager", "System", "Ctrl+Shift+Esc"),
                ("Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""),
                ("Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""),
                ("Play", "MUSIC", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""),
                ("Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"),
            ],
            "#F4B860",
            "#2A2215",
        ),
        nav_zones("FILES", "Explorer", "sys.hotkey", "#F4B860"),
    )
}

fn home_page() -> ProfilePage {
    make_page(
        "page-home",
        "Home",
        paint_keys(
            &[
                ("Desktop", "SHOW", "Monitor", "sys.show_desktop", "System: Show Desktop", "System", "Win+D"),
                ("Screenshot", "SNAP", "Camera", "sys.screenshot", "System: Screenshot", "System", "Win+Shift+S"),
                ("Files", "OPEN", "Folder", "sys.hotkey", "System: File Explorer", "System", "Win+E"),
                ("Settings", "WIN", "Settings", "sys.hotkey", "System: Windows Settings", "System", "Win+I"),
                ("Lock", "PC", "Zap", "sys.lock_pc", "System: Lock PC", "System", ""),
                ("Task Mgr", "TASK", "Terminal", "sys.hotkey", "System: Task Manager", "System", "Ctrl+Shift+Esc"),
                ("Vol Up", "+2%", "Volume2", "audio.volume_up", "Audio: Volume Up", "Media", ""),
                ("Mute Mic", "MIC", "MicOff", "audio.mute_mic", "Audio: Mute Microphone", "Media", ""),
                ("Play", "MUSIC", "Music", "media.play_pause", "Media: Play / Pause", "Media", ""),
                ("Music", "PAGE", "Music", "profile.switch_page", "Navigation: Music Page", "Navigation", "Music"),
            ],
            "#A6AFC4",
            "#1A1E27",
        ),
        nav_zones("DESK", "Show desktop", "sys.show_desktop", "#A6AFC4"),
    )
}

fn work_zone_needs_param(zones: &mut [TouchZoneConfig]) {
    for zone in zones {
        if zone.tap_action_id == "sys.hotkey" && zone.tap_action_param.is_empty() {
            zone.tap_action_param = "Win+E".to_string();
        }
    }
}

pub fn default_profiles() -> Vec<Profile> {
    let ts = now_ms();
    let stream = ProfilePage {
        id: "page-1".to_string(),
        name: "Page 1".to_string(),
        keys: default_keys(),
        touch_zones: default_touch_zones(),
        encoders: default_encoders(),
        strip_media: None,
    };
    let game = game_page();
    let edit = edit_page();
    let mut work = work_page();
    work_zone_needs_param(&mut work.touch_zones);
    let home = home_page();
    let music = music_page();
    let desktop = desktop_page();

    vec![
        Profile {
            id: "profile-streaming".to_string(),
            name: "Streaming".to_string(),
            icon: "Radio".to_string(),
            color_strip: "#8B5CF6".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![stream, desktop.clone(), music.clone()],
            last_modified_ms: ts,
        },
        Profile {
            id: "profile-gaming".to_string(),
            name: "Gaming".to_string(),
            icon: "Gamepad2".to_string(),
            color_strip: "#35C98B".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![game, music.clone(), desktop.clone()],
            last_modified_ms: ts,
        },
        Profile {
            id: "profile-editing".to_string(),
            name: "Editing".to_string(),
            icon: "Film".to_string(),
            color_strip: "#56B9F2".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![edit, music.clone(), desktop.clone()],
            last_modified_ms: ts,
        },
        Profile {
            id: "profile-work".to_string(),
            name: "Work".to_string(),
            icon: "Briefcase".to_string(),
            color_strip: "#F4B860".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![work, music.clone(), desktop.clone()],
            last_modified_ms: ts,
        },
        Profile {
            id: "profile-default".to_string(),
            name: "Default".to_string(),
            icon: "Layers".to_string(),
            color_strip: "#A6AFC4".to_string(),
            target_device_id: "ajazz-akp05e-pro".to_string(),
            pages: vec![home, music, desktop],
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
            target_page_id: String::new(),
            priority: 1,
            enabled: true,
        },
        AutoSwitchRule {
            id: "rule-resolve".to_string(),
            process_name: "Resolve.exe".to_string(),
            window_title_contains: "DaVinci Resolve".to_string(),
            target_profile_id: "profile-editing".to_string(),
            target_page_id: String::new(),
            priority: 2,
            enabled: true,
        },
        AutoSwitchRule {
            id: "rule-code".to_string(),
            process_name: "Code.exe".to_string(),
            window_title_contains: "Visual Studio Code".to_string(),
            target_profile_id: "profile-work".to_string(),
            target_page_id: String::new(),
            priority: 3,
            enabled: true,
        },
        AutoSwitchRule {
            id: "rule-spotify".to_string(),
            process_name: "Spotify.exe".to_string(),
            window_title_contains: String::new(),
            target_profile_id: "profile-streaming".to_string(),
            target_page_id: String::new(),
            priority: 4,
            enabled: false,
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
        media: None,
    }
}

/// Parses all `.sdProfile` folders in `%APPDATA%\HotSpot\StreamDock\profiles` and returns
/// rich `Profile` structs ready to use in DeckForge!
pub fn parse_all_legacy_sdprofiles() -> Vec<Profile> {
    // Without APPDATA (not Windows) there is nothing to import.
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return Vec::new();
    };
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
                media: None,
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
                strip_media: None,
            }],
            last_modified_ms: now_ms(),
        });
    }

    imported
}

/// Scans `%APPDATA%\HotSpot\StreamDock\profiles` and `%APPDATA%\HotSpot\StreamDock\plugins`.
pub fn scan_legacy_streamdock_environment() -> LegacyStreamDockSummary {
    let appdata = std::env::var_os("APPDATA").unwrap_or_default();
    let base = PathBuf::from(&appdata).join("HotSpot").join("StreamDock");
    scan_legacy_streamdock_at(&base, parse_all_legacy_sdprofiles().len())
}

fn scan_legacy_streamdock_at(base: &std::path::Path, named_root_profiles_count: usize) -> LegacyStreamDockSummary {
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
                    enabled: false, // Discovery does not establish runtime compatibility.
                    permissions: Vec::new(),
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
        plugin_ws_port: None,
        plugin_host_active: false,
    }
}

pub fn profiles_store_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("DeckForge").join("profiles.json")
}

/// The deck does not keep a layout. This file is the copy DeckForge sends after each connect.
/// Writes to a temporary file first, then swaps it in, so a crash mid-save cannot leave a
/// half-written file behind.
pub(crate) fn write_atomic(path: &std::path::Path, raw: &str) -> Result<(), String> {
    write_atomic_bytes(path, raw.as_bytes())
}

pub(crate) fn write_atomic_bytes(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let tmp = path.with_extension("json.tmp");
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Parses a saved file. One that does not parse is moved aside to `<name>.corrupt-<ms>.json`
/// before the defaults are used, so the next save cannot overwrite the user's data.
fn parse_or_quarantine<T: serde::de::DeserializeOwned>(path: &std::path::Path, raw: &str) -> Option<T> {
    match serde_json::from_str(raw) {
        Ok(value) => Some(value),
        Err(err) => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let backup = path.with_extension(format!("corrupt-{stamp}.json"));
            let _ = fs::copy(path, &backup);
            eprintln!("{} could not be read ({err}); kept a copy at {}", path.display(), backup.display());
            None
        }
    }
}

const PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileStore {
    schema_version: u32,
    profiles: Vec<Profile>,
}

// Legacy arrays are version 0. Upgrading the container must never infer edits
// to actions/pages from titles or profile IDs.
fn decode_profiles(raw: &str) -> Result<(Vec<Profile>, bool), String> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|e| format!("Invalid profile store: {e}"))?;
    if value.is_array() {
        return serde_json::from_value(value).map(|profiles| (profiles, true)).map_err(|e| e.to_string());
    }
    let version = value.get("schemaVersion").and_then(|v| v.as_u64())
        .ok_or("Profile store is missing a valid schemaVersion")?;
    if version != u64::from(PROFILE_SCHEMA_VERSION) {
        return Err(format!("Unsupported profile schema version {version}; this build supports {PROFILE_SCHEMA_VERSION}. The file was not changed."));
    }
    let store: ProfileStore = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok((store.profiles, false))
}

pub fn load_saved_profiles() -> Result<Vec<Profile>, String> {
    load_profiles_at(&profiles_store_path())
}

fn load_profiles_at(path: &std::path::Path) -> Result<Vec<Profile>, String> {
    match fs::read_to_string(path) {
        Ok(raw) => decode_profiles(&raw).map(|(profiles, _)| profiles),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(default_profiles()),
        Err(err) => Err(format!("Could not read {}: {err}", path.display())),
    }
}

fn backup_legacy_profiles(path: &std::path::Path, raw: &str) -> Result<(), String> {
    use std::io::Write;
    // create_new prevents an older backup from being overwritten, including
    // when a user restores an array file and upgrades it a second time.
    for index in 0..1000 {
        let backup = path.with_extension(format!("pre-v1-{index}.json"));
        match fs::OpenOptions::new().write(true).create_new(true).open(&backup) {
            Ok(mut file) => {
                file.write_all(raw.as_bytes()).and_then(|_| file.sync_all())
                    .map_err(|e| format!("Could not preserve legacy profiles at {}: {e}", backup.display()))?;
                return Ok(());
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("Could not back up legacy profiles: {err}")),
        }
    }
    Err("No available legacy profile backup filename; original file was not changed".into())
}

fn write_profiles_at(path: &std::path::Path, profiles: &[Profile]) -> Result<(), String> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    // Do not overwrite a malformed or newer-format store with fallback defaults.
    match fs::read_to_string(path) {
        Ok(raw) => {
            let (_, legacy) = decode_profiles(&raw)?;
            if legacy { backup_legacy_profiles(path, &raw)?; }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {},
        Err(err) => return Err(format!("Could not inspect existing profiles before save: {err}")),
    }
    let store = ProfileStore { schema_version: PROFILE_SCHEMA_VERSION, profiles: profiles.to_vec() };
    let raw = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    write_atomic(path, &raw)
}

pub fn auto_switch_store_path() -> PathBuf {
    profiles_store_path().with_file_name("auto-switch.json")
}

pub fn load_auto_switch_rules() -> Vec<AutoSwitchRule> {
    let path = auto_switch_store_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        return default_auto_switch_rules();
    };
    parse_or_quarantine(&path, &raw).unwrap_or_else(default_auto_switch_rules)
}

pub fn write_auto_switch_rules(rules: &[AutoSwitchRule]) -> Result<(), String> {
    let path = auto_switch_store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(rules).map_err(|e| e.to_string())?;
    write_atomic(&path, &raw)
}

pub fn macros_store_path() -> PathBuf {
    profiles_store_path().with_file_name("macros.json")
}

pub fn load_saved_macros() -> Vec<MacroSequence> {
    let path = macros_store_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        let macros = default_macros();
        let _ = write_saved_macros(&macros);
        return macros;
    };
    parse_or_quarantine(&path, &raw).unwrap_or_else(default_macros)
}

pub fn write_saved_macros(macros: &[MacroSequence]) -> Result<(), String> {
    let path = macros_store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(macros).map_err(|e| e.to_string())?;
    write_atomic(&path, &raw)
}

pub fn write_saved_profiles(profiles: &[Profile]) -> Result<(), String> {
    write_profiles_at(&profiles_store_path(), profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_discovery_never_claims_vendor_runtime_support() {
        let root = std::env::temp_dir().join(format!("deckforge-plugin-test-{}-{}", std::process::id(), now_ms()));
        let package = root.join("plugins/example.sdPlugin");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("manifest.json"), r#"{"Name":"Example","Actions":[{"Name":"Vendor action"}]}"#).unwrap();
        fs::create_dir_all(root.join("plugins/broken.sdPlugin")).unwrap();
        fs::write(root.join("plugins/broken.sdPlugin/manifest.json"), "invalid json").unwrap();
        let summary = scan_legacy_streamdock_at(&root, 0);
        assert!(!summary.plugin_host_active);
        assert_eq!(summary.plugin_ws_port, None);
        assert_eq!(summary.legacy_plugin_count, 2);
        let vendor: Vec<_> = summary.discovered_plugins.iter().filter(|p| p.source_kind == "LocalSdPlugin").collect();
        assert_eq!(vendor.len(), 2);
        assert!(vendor.iter().all(|p| !p.enabled && p.permissions.is_empty() && p.disk_path.is_some()));
        let example = vendor.iter().find(|p| p.name == "Example").unwrap();
        assert_eq!(example.actions_provided, vec!["Vendor action"]);
        assert!(summary.discovered_plugins.iter().filter(|p| p.source_kind == "BuiltIn").all(|p| p.enabled));
        fs::remove_dir_all(&root).unwrap();
        let empty = scan_legacy_streamdock_at(&root, 0);
        assert_eq!(empty.legacy_plugin_count, 0);
        assert!(empty.discovered_plugins.iter().all(|p| p.source_kind == "BuiltIn"));
    }

    struct ProfileFixture(PathBuf);
    impl ProfileFixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!("deckforge-profile-test-{}-{}-{serial}", std::process::id(), now_ms()));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn path(&self) -> PathBuf { self.0.join("profiles.json") }
    }
    impl Drop for ProfileFixture {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    fn custom_profiles() -> Vec<Profile> {
        let mut profiles = default_profiles();
        // Same stock titles/ID that used to trigger wholesale replacement,
        // but with customized bindings, and deliberately absent stock pages.
        let mut stream = profiles[0].pages[0].clone();
        stream.keys[0].action_id = "sys.hotkey".into();
        stream.keys[0].action_param = "Ctrl+Alt+F9".into();
        stream.keys[1].action_id = "profile.switch_page".into();
        stream.keys[1].action_param = "Page 2 Custom".into();
        stream.touch_zones[0].tap_action_id = "sys.hotkey".into();
        stream.touch_zones[0].tap_action_param.clear();
        stream.touch_zones[0].label = "CPU LOAD".into();
        stream.touch_zones[0].value_text = "Ctrl+F8".into();
        profiles[1].pages = vec![stream.clone()];
        let mut telemetry = stream.clone();
        telemetry.name = "Page 2 (Telemetry)".into();
        telemetry.keys[0].title = "CPU Monitor".into();
        telemetry.keys[0].action_id = "obs.start_stream".into();
        profiles[0].pages = vec![stream, telemetry];
        profiles[2].pages.retain(|p| p.name != "Desktop");
        profiles[2].pages.iter_mut().filter(|p| p.name == "Music").for_each(|p| p.name = "My Music".into());
        profiles
    }

    #[test]
    fn legacy_startup_preserves_custom_actions_titles_deleted_and_renamed_pages() {
        let fixture = ProfileFixture::new(); let path = fixture.path();
        let profiles = custom_profiles();
        let raw = serde_json::to_string_pretty(&profiles).unwrap();
        fs::write(&path, &raw).unwrap();
        for _ in 0..3 {
            let loaded = load_profiles_at(&path).unwrap();
            assert_eq!(serde_json::to_value(&loaded).unwrap(), serde_json::to_value(&profiles).unwrap());
            assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        }
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1, "startup must not write backups or rewrite storage");
    }

    #[test]
    fn explicit_save_upgrades_container_once_and_keeps_original_bytes() {
        let fixture = ProfileFixture::new(); let path = fixture.path();
        let mut profiles = custom_profiles();
        let original = serde_json::to_string(&profiles).unwrap();
        fs::write(&path, &original).unwrap();
        profiles[0].name = "Edited after loading".into();
        write_profiles_at(&path, &profiles).unwrap();
        assert_eq!(fs::read_to_string(path.with_extension("pre-v1-0.json")).unwrap(), original);
        for _ in 0..3 {
            let loaded = load_profiles_at(&path).unwrap();
            assert_eq!(serde_json::to_value(&loaded).unwrap(), serde_json::to_value(&profiles).unwrap());
            write_profiles_at(&path, &loaded).unwrap();
        }
        let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 2, "upgraded files need no repeat migration backups");
    }

    #[test]
    fn existing_backup_is_never_overwritten() {
        let fixture = ProfileFixture::new(); let path = fixture.path();
        fs::write(path.with_extension("pre-v1-0.json"), "older original").unwrap();
        fs::write(&path, "[]").unwrap();
        write_profiles_at(&path, &[]).unwrap();
        assert_eq!(fs::read_to_string(path.with_extension("pre-v1-0.json")).unwrap(), "older original");
        assert_eq!(fs::read_to_string(path.with_extension("pre-v1-1.json")).unwrap(), "[]");
        assert!(load_profiles_at(&path).unwrap().is_empty());
    }

    #[test]
    fn invalid_and_future_stores_cannot_be_overwritten_by_defaults() {
        let fixture = ProfileFixture::new(); let path = fixture.path();
        for raw in ["broken json", r#"{"schemaVersion":2,"profiles":[]}"#,
            r#"{"schemaVersion":1,"profiles":[{}]}"#, r#"{"profiles":[]}"#] {
            fs::write(&path, raw).unwrap();
            assert!(load_profiles_at(&path).is_err());
            assert!(write_profiles_at(&path, &default_profiles()).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        }
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    }

    #[test]
    fn failed_backup_or_replacement_preserves_legacy_store() {
        let fixture = ProfileFixture::new(); let path = fixture.path();
        fs::write(&path, "[]").unwrap();
        fs::create_dir(path.with_extension("json.tmp")).unwrap();
        assert!(write_profiles_at(&path, &default_profiles()).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "[]");
        assert_eq!(fs::read_to_string(path.with_extension("pre-v1-0.json")).unwrap(), "[]");
        fs::remove_dir(path.with_extension("json.tmp")).unwrap();
        for index in 1..1000 { fs::write(path.with_extension(format!("pre-v1-{index}.json")), "old backup").unwrap(); }
        assert!(write_profiles_at(&path, &default_profiles()).unwrap_err().contains("backup filename"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "[]");
    }

    #[test]
    fn first_run_defaults_are_not_written_until_explicit_save() {
        let fixture = ProfileFixture::new(); let path = fixture.path();
        let defaults = load_profiles_at(&path).unwrap();
        assert!(!defaults.is_empty());
        assert!(!path.exists());
        write_profiles_at(&path, &defaults).unwrap();
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
        assert_eq!(load_profiles_at(&path).unwrap().len(), defaults.len());
    }

}
