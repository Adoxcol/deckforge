pub mod actions;
pub mod db;
pub mod drivers;
pub mod game_mode;
pub mod media;

use actions::{
    collect_system_telemetry, execute_key_action_live, foreground_window, launch_app_or_url,
    macro_delay_ms, process_is_running, read_audio_face, read_media_face, send_hotkey_chord,
    stop_official_streamdock_app, trigger_http_webhook, ActionExecutionResult, MacroRunReport,
    ObsSettings, SystemTelemetrySnapshot,
};
use db::{
    load_auto_switch_rules, load_saved_macros, load_saved_profiles, parse_all_legacy_sdprofiles,
    scan_legacy_streamdock_environment, write_auto_switch_rules, write_saved_macros, write_saved_profiles,
    AutoSwitchRule,
    KeyConfig, LegacyStreamDockSummary, MacroSequence, Profile, ProfilePage, TouchZoneConfig,
};
use drivers::akp05e_protocol::{
    build_key_image_packets, build_touch_strip_packets, parse_input_report, resolve_strip_point,
    HardwareInputEvent, InputControlKind, StripGesture,
    encode_procedural_key_jpeg, encode_touch_strip_jpeg,
};
use drivers::frame_cache::{AntiFreezeFrameCache, FrameCacheStats};
use drivers::hal::{
    builtin_device_catalog, clear_hardware_key, deck_generation, deck_next_report,
    device_uses_akp05e_face, get_cache_stats, key_image_slot, now_ms, publish_screens,
    scan_usb_devices, send_hardware_brightness, sync_keys_to_hardware,
    touch_zone_image_slot, write_animation_frames, write_hid_bursts, write_one_report,
    write_panel_brightness, wake_panel, deck_note, request_full_push, DeviceDescriptor, DiagnosticEntry,
    HidBurst, PlaybackSlot,
    PlaybackTarget, ScreenJob, StripJob, SyncKeyResult, ZoneJob, AKP05E_TOUCH_WINDOW_H,
    AKP05E_TOUCH_WINDOW_W,
};
use media::{decode_b64, MediaPreview, MediaStore};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullBootstrapPayload {
    pub devices: Vec<DeviceDescriptor>,
    pub profiles: Vec<Profile>,
    pub auto_switch_rules: Vec<AutoSwitchRule>,
    pub macros: Vec<MacroSequence>,
    pub diagnostics: Vec<DiagnosticEntry>,
    pub frame_cache_stats: FrameCacheStats,
    pub legacy_summary: LegacyStreamDockSummary,
    pub telemetry: SystemTelemetrySnapshot,
    pub saved_locally_at_ms: u64,
    pub applied_to_device_at_ms: Option<u64>,
    pub recent_input_events: Vec<HardwareInputEvent>,
    pub media_library: Vec<MediaPreview>,
    pub face: DeckFaceState,
    pub obs: ObsSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeckFaceState {
    pub media_playing: Option<bool>,
    pub media_title: String,
    pub mic_muted: Option<bool>,
    pub speaker_muted: Option<bool>,
    pub volume_percent: Option<u8>,
    pub game_mode: Option<bool>,
    pub game_mode_busy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckContextEvent {
    pub profile_id: String,
    pub page_index: usize,
    pub message: String,
}

pub struct ManagedAppState {
    pub devices: Vec<DeviceDescriptor>,
    pub profiles: Vec<Profile>,
    pub active_profile_id: String,
    pub active_page_index: usize,
    pub auto_switch_rules: Vec<AutoSwitchRule>,
    pub macros: Vec<MacroSequence>,
    pub diagnostics: Vec<DiagnosticEntry>,
    pub recent_input_events: Vec<HardwareInputEvent>,
    pub frame_cache: AntiFreezeFrameCache,
    pub saved_locally_at_ms: u64,
    pub applied_to_device_at_ms: Option<u64>,
    pub media: MediaStore,
    pub playback_slots: Vec<PlaybackSlot>,
    pub playback_device_id: String,
    pub usb_bus: Arc<Mutex<()>>,
    pub face: DeckFaceState,
    pub obs: ObsSettings,
    pub sample_after_ms: u64,
    pub auto_switch_hold_until_ms: u64,
}

impl Default for ManagedAppState {
    fn default() -> Self {
        let mut devices = builtin_device_catalog();
        let mut diagnostics = vec![DiagnosticEntry {
            id: "diag-init".to_string(),
            timestamp_ms: now_ms(),
            level: "info".to_string(),
            device_id: "system".to_string(),
            stage: "BOOT".to_string(),
            message: "DeckForge Rust Hardware & Action Service initialized (SHA-256 Frame Diffing + Win32 Input Engine enabled)".to_string(),
            packet_hex: None,
        }];
        scan_usb_devices(&mut devices, &mut diagnostics);

        let profiles = load_saved_profiles();
        let frame_cache = AntiFreezeFrameCache::new();
        let applied_to_device_at_ms = None;

        for d in &devices {
            if d.connected {
                diagnostics.push(DiagnosticEntry {
                    id: format!("diag-usb-{}", d.id),
                    timestamp_ms: now_ms(),
                    level: "success".to_string(),
                    device_id: d.id.clone(),
                    stage: "USB_DETECTED".to_string(),
                    message: format!(
                        "Verified hardware detected: {} ({}) Serial: {}",
                        d.model_name, d.vid_pid_hex, d.serial_number
                    ),
                    packet_hex: Some("CRT\\0\\0HAN ready (1024B payload)".to_string()),
                });
            }
        }

        Self {
            devices,
            profiles,
            active_profile_id: "profile-streaming".to_string(),
            active_page_index: 0,
            auto_switch_rules: load_auto_switch_rules(),
            macros: load_saved_macros(),
            diagnostics,
            recent_input_events: Vec::new(),
            frame_cache,
            saved_locally_at_ms: now_ms(),
            applied_to_device_at_ms,
            media: MediaStore::load(),
            playback_slots: Vec::new(),
            playback_device_id: String::new(),
            usb_bus: Arc::new(Mutex::new(())),
            face: DeckFaceState::default(),
            obs: ObsSettings::load(),
            sample_after_ms: 0,
            auto_switch_hold_until_ms: 0,
        }
    }
}

pub type SharedState = Arc<Mutex<ManagedAppState>>;

struct LiveAction {
    action_id: String,
    label: String,
    param: String,
    note: String,
    device_id: String,
}

fn kind_code(kind: &InputControlKind) -> u8 {
    match kind {
        InputControlKind::LcdKey => 1,
        InputControlKind::RotaryKnobTurnCw => 2,
        InputControlKind::RotaryKnobTurnCcw => 3,
        InputControlKind::RotaryKnobPush => 4,
        InputControlKind::TouchStripTap => 5,
        InputControlKind::TouchStripSwipe => 6,
        InputControlKind::TouchStripPoint => 7,
        InputControlKind::Unknown => 0,
    }
}

fn action_for_event(guard: &ManagedAppState, evt: &HardwareInputEvent) -> Option<LiveAction> {
    let prof = guard
        .profiles
        .iter()
        .find(|p| p.id == guard.active_profile_id)
        .or_else(|| guard.profiles.first())?;
    let page = prof
        .pages
        .get(guard.active_page_index)
        .or_else(|| prof.pages.first())?;
    match evt.kind {
        InputControlKind::LcdKey => {
            let key = page.keys.get(evt.control_index as usize)?;
            Some(LiveAction {
                action_id: key.action_id.clone(),
                label: key.action_label.clone(),
                param: key.action_param.clone(),
                note: format!("Key #{} ({})", evt.control_index + 1, key.title),
                device_id: evt.device_id.clone(),
            })
        }
        InputControlKind::RotaryKnobTurnCw => {
            let encoder = page.encoders.get(evt.control_index as usize);
            Some(binding_action(
                encoder.map(|e| e.cw_action.as_str()).unwrap_or("audio.volume_up"),
                encoder.map(|e| e.cw_param.as_str()).unwrap_or(""),
                "Knob clockwise",
                format!("Knob #{} clockwise", evt.control_index + 1),
                evt.device_id.clone(),
            ))
        }
        InputControlKind::RotaryKnobTurnCcw => {
            let encoder = page.encoders.get(evt.control_index as usize);
            Some(binding_action(
                encoder.map(|e| e.ccw_action.as_str()).unwrap_or("audio.volume_down"),
                encoder.map(|e| e.ccw_param.as_str()).unwrap_or(""),
                "Knob counter-clockwise",
                format!("Knob #{} counter-clockwise", evt.control_index + 1),
                evt.device_id.clone(),
            ))
        }
        InputControlKind::RotaryKnobPush => {
            let encoder = page.encoders.get(evt.control_index as usize);
            Some(binding_action(
                encoder.map(|e| e.push_action.as_str()).unwrap_or("audio.volume_mute"),
                encoder.map(|e| e.push_param.as_str()).unwrap_or(""),
                "Knob press",
                format!("Knob #{} press", evt.control_index + 1),
                evt.device_id.clone(),
            ))
        }
        InputControlKind::TouchStripTap => {
            let zone = page.touch_zones.get(evt.control_index as usize)?;
            Some(LiveAction {
                action_id: zone.tap_action_id.clone(),
                label: zone.label.clone(),
                param: zone.tap_action_param.clone(),
                note: format!("Touch zone {} ({})", evt.control_index + 1, zone.label),
                device_id: evt.device_id.clone(),
            })
        }
        InputControlKind::TouchStripSwipe => {
            let next = evt.control_index != 0;
            Some(LiveAction {
                action_id: if next { "profile.next_page" } else { "profile.prev_page" }.to_string(),
                label: if next { "Next page" } else { "Previous page" }.to_string(),
                param: String::new(),
                note: if next { "Swipe right" } else { "Swipe left" }.to_string(),
                device_id: evt.device_id.clone(),
            })
        }
        InputControlKind::TouchStripPoint | InputControlKind::Unknown => None,
    }
}

fn known_action(binding: &str) -> bool {
    matches!(
        binding,
        "media.play_pause"
            | "media.next"
            | "media.prev"
            | "media.stop"
            | "audio.volume_up"
            | "audio.volume_down"
            | "audio.volume_mute"
            | "audio.mute_mic"
            | "audio.open_mixer"
            | "sys.show_desktop"
            | "sys.screenshot"
            | "sys.task_manager"
            | "sys.lock_pc"
            | "sys.hotkey"
            | "custom.hotkey"
            | "custom.launch"
            | "custom.shell"
            | "custom.url"
            | "custom.text"
            | "custom.http"
            | "obs.start_stream"
            | "obs.next_scene"
            | "obs.prev_scene"
            | "obs.toggle_record"
            | "obs.save_replay"
            | "brightness.up"
            | "brightness.down"
            | "brightness.toggle"
            | "profile.next_page"
            | "profile.prev_page"
            | "profile.switch_page"
            | "profile.switch"
            | "macro.run"
            | "sys.game_mode"
    )
}

fn binding_action(binding: &str, param: &str, label: &str, note: String, device_id: String) -> LiveAction {
    if known_action(binding) {
        LiveAction {
            action_id: binding.to_string(),
            label: label.to_string(),
            param: param.to_string(),
            note,
            device_id,
        }
    } else {
        LiveAction {
            action_id: "custom.hotkey".to_string(),
            label: label.to_string(),
            param: binding.to_string(),
            note,
            device_id,
        }
    }
}

fn face_changes(action_id: &str) -> bool {
    matches!(
        action_id,
        "media.play_pause"
            | "media.next"
            | "media.prev"
            | "media.stop"
            | "audio.volume_up"
            | "audio.volume_down"
            | "audio.volume_mute"
            | "audio.mute_mic"
            | "sys.game_mode"
    )
}

fn lcd_title(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if out.chars().count() >= 12 {
            break;
        }
        if ch.is_ascii_alphanumeric() || " .'-&+".contains(ch) {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push(' ');
        }
    }
    let compact = out.split_whitespace().collect::<Vec<_>>().join(" ");
    compact
}

fn key_level(action_id: &str, face: &DeckFaceState) -> Option<u8> {
    if matches!(action_id, "audio.volume_up" | "audio.volume_down" | "audio.volume_mute") {
        face.volume_percent
    } else {
        None
    }
}

fn styled_key(key: &KeyConfig, face: &DeckFaceState) -> (String, String) {
    if key.media.is_some() {
        return (key.title.clone(), key.icon_name.clone());
    }
    match key.action_id.as_str() {
        "media.play_pause" => {
            if face.media_playing.is_none() && face.media_title.trim().is_empty() {
                return (key.title.clone(), key.icon_name.clone());
            }
            let title = if face.media_title.trim().is_empty() {
                if face.media_playing == Some(true) { "Playing" } else { "Paused" }.to_string()
            } else {
                let label = lcd_title(&face.media_title);
                if label.is_empty() { "Playing".to_string() } else { label }
            };
            let icon = if face.media_playing == Some(true) { "Pause" } else { "Play" };
            (title, icon.to_string())
        }
        "sys.game_mode" => {
            if face.game_mode_busy {
                ("Working".to_string(), "Zap".to_string())
            } else if face.game_mode == Some(true) {
                ("GAME ON".to_string(), "Zap".to_string())
            } else {
                (key.title.clone(), key.icon_name.clone())
            }
        }
        "audio.mute_mic" => match face.mic_muted {
            Some(true) => ("Muted".to_string(), "MicOff".to_string()),
            Some(false) => ("Mic Live".to_string(), "Mic".to_string()),
            None => (key.title.clone(), key.icon_name.clone()),
        },
        "audio.volume_up" | "audio.volume_down" | "audio.volume_mute" => match face.volume_percent {
            Some(percent) => {
                let title = if face.speaker_muted == Some(true) {
                    "Muted".to_string()
                } else {
                    format!("{percent}%")
                };
                (title, key.icon_name.clone())
            }
            None => (key.title.clone(), key.icon_name.clone()),
        },
        _ => (key.title.clone(), key.icon_name.clone()),
    }
}

fn styled_zone_value(zone: &TouchZoneConfig, face: &DeckFaceState) -> String {
    if zone.media.is_some() {
        return zone.value_text.clone();
    }
    match zone.tap_action_id.as_str() {
        "media.play_pause" => match face.media_playing {
            Some(true) => {
                if face.media_title.is_empty() {
                    "Playing".to_string()
                } else {
                    face.media_title.clone()
                }
            }
            Some(false) => "Paused".to_string(),
            None => zone.value_text.clone(),
        },
        "audio.mute_mic" => match face.mic_muted {
            Some(true) => "Muted".to_string(),
            Some(false) => "Live".to_string(),
            None => zone.value_text.clone(),
        },
        "audio.volume_mute" | "audio.volume_up" | "audio.volume_down" => match face.volume_percent {
            Some(percent) => {
                if face.speaker_muted == Some(true) {
                    format!("{percent}% muted")
                } else {
                    format!("{percent}%")
                }
            }
            None => zone.value_text.clone(),
        },
        _ => zone.value_text.clone(),
    }
}

fn plan_screens(dev: &DeviceDescriptor, page: &ProfilePage, face: &DeckFaceState) -> (Vec<ScreenJob>, Vec<ZoneJob>, Option<StripJob>) {
    let key_count = dev.keys.count as usize;
    let jobs = page
        .keys
        .iter()
        .take(key_count)
        .map(|key| {
            let (title, icon_name) = styled_key(key, face);
            ScreenJob {
                index: key.index,
                title,
                show_title: key.show_title,
                icon_name,
                accent: key.accent_color.clone(),
                bg: key.bg_color.clone(),
                media_id: key.media.as_ref().map(|m| m.media_id.clone()),
                fit: key.media.as_ref().map(|m| m.fit.clone()).unwrap_or_else(|| "cover".into()),
                speed: key.media.as_ref().map(|m| m.speed).unwrap_or(1.0),
                loop_playback: key.media.as_ref().map(|m| m.loop_playback).unwrap_or(true),
                level: if key.media.is_some() { None } else { key_level(&key.action_id, face) },
            }
        })
        .collect();
    let zones = page
        .touch_zones
        .iter()
        .map(|zone| ZoneJob {
            label: zone.label.clone(),
            value_text: styled_zone_value(zone, face),
            accent: zone.accent_color.clone(),
            media_id: zone.media.as_ref().map(|m| m.media_id.clone()),
            fit: zone.media.as_ref().map(|m| m.fit.clone()).unwrap_or_else(|| "cover".into()),
            speed: zone.media.as_ref().map(|m| m.speed).unwrap_or(1.0),
            loop_playback: zone.media.as_ref().map(|m| m.loop_playback).unwrap_or(true),
        })
        .collect();
    let strip = page.strip_media.as_ref().map(|m| StripJob {
        media_id: m.media_id.clone(),
        fit: m.fit.clone(),
        speed: m.speed,
        loop_playback: m.loop_playback,
    });
    (jobs, zones, strip)
}

fn apply_brightness_action(
    shared: &SharedState,
    usb: &Arc<Mutex<()>>,
    action_id: &str,
) -> Option<String> {
    if !matches!(action_id, "brightness.up" | "brightness.down" | "brightness.toggle") {
        return None;
    }
    let (dev, percent) = {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        let dev = guard
            .devices
            .iter_mut()
            .find(|d| d.connected && d.supports_brightness)?;
        let next = match action_id {
            "brightness.up" => dev.brightness.saturating_add(5).min(100),
            "brightness.down" => dev.brightness.saturating_sub(5),
            _ => {
                if dev.brightness == 0 {
                    95
                } else {
                    0
                }
            }
        };
        dev.brightness = next;
        (dev.clone(), next)
    };
    let _bus = usb.lock().unwrap_or_else(|e| e.into_inner());
    let _ = write_panel_brightness(&dev, percent);
    Some(format!("Backlight set to {percent}%"))
}

/// Streams the saved layout after the deck has accepted the handshake.
/// These panels keep no picture of their own; a later handshake leaves them black until this runs.
fn push_connected_layout(shared: &SharedState, usb: &Arc<Mutex<()>>) {
    {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(dev) = guard
            .devices
            .iter()
            .find(|d| d.connected && d.protocol_family.starts_with("CRT"))
        {
            if guard.profiles.iter().any(|p| p.id == dev.assigned_profile_id) {
                guard.active_profile_id = dev.assigned_profile_id.clone();
            }
            guard.active_page_index = 0;
        }
    }
    // First contact may find the panel blank, so this one path handshakes and
    // then restores brightness. Later page changes must not handshake.
    push_active_layout(shared, usb, true);
}

fn push_active_layout(shared: &SharedState, usb: &Arc<Mutex<()>>, handshake: bool) {
    let prepared = {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        prepare_layout_bursts(&mut guard, handshake)
    };
    let Some((dev, bursts)) = prepared else {
        return;
    };
    if bursts.is_empty() {
        return;
    }
    let _bus = usb.lock().unwrap_or_else(|e| e.into_inner());
    if handshake {
        let _ = write_hid_bursts(&dev, &bursts);
    } else {
        // A page change only replaces the pictures. HAN would fade the backlight off.
        let _ = write_animation_frames(&dev, &bursts);
    }
}

fn prepare_layout_bursts(guard: &mut ManagedAppState, invalidate: bool) -> Option<(DeviceDescriptor, Vec<HidBurst>)> {
    let dev = guard
        .devices
        .iter()
        .find(|d| d.connected && d.protocol_family.starts_with("CRT"))
        .cloned()?;
    if invalidate {
        // A handshake blanks the panels, so the next send has to include every screen.
        guard.frame_cache.invalidate_device(&dev.id);
    }
    let profile_id = guard.active_profile_id.clone();
    let prof = guard
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .or_else(|| guard.profiles.first())?
        .clone();
    let page = prof
        .pages
        .get(guard.active_page_index)
        .or_else(|| prof.pages.first())?
        .clone();
    let (jobs, zones, strip) = plan_screens(&dev, &page, &guard.face);
    let media = std::mem::take(&mut guard.media);
    let mut diags = Vec::new();
    let outcome = publish_screens(
        &dev,
        &jobs,
        &zones,
        strip.as_ref(),
        &media,
        &mut guard.frame_cache,
        &mut diags,
    );
    guard.media = media;
    guard.diagnostics.extend(diags);
    if guard.diagnostics.len() > 80 {
        let extra = guard.diagnostics.len() - 80;
        guard.diagnostics.drain(0..extra);
    }
    guard.playback_slots = outcome.playback;
    guard.playback_device_id = dev.id.clone();
    guard.active_profile_id = prof.id.clone();
    guard.applied_to_device_at_ms = Some(now_ms());
    Some((dev, outcome.bursts))
}

fn dispatch_live_action(
    app: &tauri::AppHandle,
    shared: &SharedState,
    usb: &Arc<Mutex<()>>,
    action: &LiveAction,
) -> String {
    if action.action_id == "macro.run" {
        return start_macro(app, shared, usb, &action.param, &action.label);
    }
    if action.action_id == "sys.game_mode" {
        return start_game_mode_toggle(app, shared, &action.param);
    }
    if let Some(message) = apply_brightness_action(shared, usb, &action.action_id) {
        return message;
    }
    if let Some(message) = apply_navigation(app, shared, usb, &action.action_id, &action.param) {
        return message;
    }
    let obs = shared
        .lock()
        .ok()
        .map(|guard| guard.obs.clone())
        .unwrap_or_default();
    let result = execute_key_action_live(&action.action_id, &action.label, &action.param, true, &obs);
    if face_changes(&action.action_id) {
        if let Ok(mut guard) = shared.lock() {
            guard.sample_after_ms = now_ms().saturating_add(160);
        }
    }
    result.output_message
}

/// Nudges the key faces so the Game Mode key flips to GAME ON or back quickly.
fn nudge_face_sample(shared: &SharedState) {
    if let Ok(mut guard) = shared.lock() {
        guard.sample_after_ms = now_ms().saturating_add(120);
    }
}

/// Game Mode closes apps one at a time, so it never runs on the input thread. The key's
/// parameter picks the mode by id; an empty one uses the mode chosen in the Game Mode tab.
fn start_game_mode_toggle(app: &tauri::AppHandle, shared: &SharedState, mode: &str) -> String {
    if game_mode::is_busy() {
        return "Game Mode is still working".to_string();
    }
    let turning_on = !game_mode::is_active();
    let app = app.clone();
    let shared_for_thread = Arc::clone(shared);
    let mode = mode.trim().to_string();
    std::thread::spawn(move || {
        let message = game_mode::toggle(&app, Some(mode.as_str()).filter(|m| !m.is_empty()));
        nudge_face_sample(&shared_for_thread);
        let _ = app.emit("game-mode-message", &message);
    });
    nudge_face_sample(shared);
    if turning_on {
        "Game Mode starting".to_string()
    } else {
        "Game Mode ending".to_string()
    }
}

fn apply_navigation(
    app: &tauri::AppHandle,
    shared: &SharedState,
    usb: &Arc<Mutex<()>>,
    action_id: &str,
    param: &str,
) -> Option<String> {
    if !matches!(
        action_id,
        "profile.next_page" | "profile.prev_page" | "profile.switch_page" | "profile.switch"
    ) {
        return None;
    }
    let event = {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        let message = switch_active_page(&mut guard, action_id, param)?;
        DeckContextEvent {
            profile_id: guard.active_profile_id.clone(),
            page_index: guard.active_page_index,
            message,
        }
    };
    push_active_layout(shared, usb, false);
    let _ = app.emit("deck-context", &event);
    Some(event.message)
}

fn switch_active_page(guard: &mut ManagedAppState, action_id: &str, param: &str) -> Option<String> {
    match action_id {
        "profile.switch" => {
            let profile = guard
                .profiles
                .iter()
                .find(|profile| {
                    profile.id.eq_ignore_ascii_case(param.trim())
                        || profile.name.eq_ignore_ascii_case(param.trim())
                })?
                .clone();
            let name = profile.name.clone();
            guard.active_profile_id = profile.id;
            guard.active_page_index = 0;
            Some(format!("Switched to {name}"))
        }
        "profile.next_page" | "profile.prev_page" | "profile.switch_page" => {
            let pages = guard
                .profiles
                .iter()
                .find(|profile| profile.id == guard.active_profile_id)
                .or_else(|| guard.profiles.first())?
                .pages
                .len();
            if pages == 0 {
                return None;
            }
            let next = match action_id {
                "profile.next_page" => (guard.active_page_index + 1) % pages,
                "profile.prev_page" => guard.active_page_index.checked_sub(1).unwrap_or(pages - 1),
                _ => resolve_page_index(guard, param).unwrap_or(guard.active_page_index),
            };
            guard.active_page_index = next;
            let page_name = guard
                .profiles
                .iter()
                .find(|profile| profile.id == guard.active_profile_id)
                .and_then(|profile| profile.pages.get(next))
                .map(|page| page.name.clone())
                .unwrap_or_else(|| format!("Page {}", next + 1));
            Some(format!("Opened {page_name}"))
        }
        _ => None,
    }
}

fn resolve_page_index(guard: &ManagedAppState, param: &str) -> Option<usize> {
    let param = param.trim();
    if param.is_empty() {
        return None;
    }
    let profile = guard
        .profiles
        .iter()
        .find(|profile| profile.id == guard.active_profile_id)
        .or_else(|| guard.profiles.first())?;
    if let Ok(number) = param.parse::<usize>() {
        if (1..=profile.pages.len()).contains(&number) {
            return Some(number - 1);
        }
    }
    let lower = param.to_lowercase();
    profile.pages.iter().position(|page| {
        page.id.eq_ignore_ascii_case(param)
            || page.name.eq_ignore_ascii_case(param)
            || page.name.to_lowercase().starts_with(&lower)
    })
}

fn sample_face(guard: &mut ManagedAppState) -> bool {
    let (playing, title) = read_media_face();
    let audio = read_audio_face();
    let next = DeckFaceState {
        media_playing: playing,
        media_title: title,
        mic_muted: audio.mic_muted,
        speaker_muted: audio.speaker_muted,
        volume_percent: audio.volume_percent,
        game_mode: Some(game_mode::is_active()),
        game_mode_busy: game_mode::is_busy(),
    };
    if next == guard.face {
        false
    } else {
        guard.face = next;
        true
    }
}

fn push_face_deltas(shared: &SharedState, usb: &Arc<Mutex<()>>, app: &tauri::AppHandle) {
    let face = {
        let guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        let _ = app.emit("deck-face", &guard.face);
        guard.face.clone()
    };
    let prepared = {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        face_delta_bursts(&mut guard, &face)
    };
    let Some((dev, bursts)) = prepared else {
        return;
    };
    if bursts.is_empty() {
        return;
    }
    let _bus = usb.lock().unwrap_or_else(|e| e.into_inner());
    let _ = write_animation_frames(&dev, &bursts);
}

fn face_delta_bursts(guard: &mut ManagedAppState, face: &DeckFaceState) -> Option<(DeviceDescriptor, Vec<HidBurst>)> {
    let dev = guard
        .devices
        .iter()
        .find(|d| d.connected && d.protocol_family.starts_with("CRT"))
        .cloned()?;
    let profile = guard
        .profiles
        .iter()
        .find(|profile| profile.id == guard.active_profile_id)
        .or_else(|| guard.profiles.first())?
        .clone();
    let page = profile
        .pages
        .get(guard.active_page_index)
        .or_else(|| profile.pages.first())?
        .clone();
    let strip_covers_zones = page.strip_media.is_some();
    let mut bursts = Vec::new();
    let key_count = dev.keys.count as usize;
    for key in page.keys.iter().take(key_count) {
        if key.media.is_some() {
            continue;
        }
        let level = key_level(&key.action_id, face);
        let (title, icon) = styled_key(key, face);
        if title == key.title && icon == key.icon_name && level.is_none() {
            continue;
        }
        let jpeg = encode_procedural_key_jpeg(
            dev.keys.pixel_width,
            dev.keys.pixel_height,
            &key.accent_color,
            &key.bg_color,
            &icon,
            if key.show_title { title.as_str() } else { "" },
            key.index,
            dev.keys.rotation,
            level,
        );
        let mut hash = jpeg.clone();
        hash.extend_from_slice(title.as_bytes());
        hash.extend_from_slice(icon.as_bytes());
        let slot = format!("key-{}", key.index);
        if guard.frame_cache.should_transmit(&dev.id, &slot, &hash).is_none() {
            continue;
        }
        bursts.push(HidBurst {
            reports: build_key_image_packets(key_image_slot(&dev, key.index), &jpeg)
                .into_iter()
                .map(|report| report.to_vec())
                .collect(),
        });
    }
    if device_uses_akp05e_face(&dev) && !strip_covers_zones {
        for zone in &page.touch_zones {
            if zone.media.is_some() {
                continue;
            }
            let value = styled_zone_value(zone, face);
            if value == zone.value_text {
                continue;
            }
            let jpeg = encode_touch_strip_jpeg(
                AKP05E_TOUCH_WINDOW_W,
                AKP05E_TOUCH_WINDOW_H,
                &[(zone.label.clone(), value.clone(), zone.accent_color.clone())],
                dev.keys.rotation,
            );
            let slot = format!("touch-zone-{}", zone.index);
            if guard.frame_cache.should_transmit(&dev.id, &slot, &jpeg).is_none() {
                continue;
            }
            bursts.push(HidBurst {
                reports: build_key_image_packets(touch_zone_image_slot(&dev, zone.index), &jpeg)
                    .into_iter()
                    .map(|report| report.to_vec())
                    .collect(),
            });
        }
    }
    Some((dev, bursts))
}

fn rule_matches(rule: &AutoSwitchRule, exe: &str, title: &str) -> bool {
    if !rule.enabled {
        return false;
    }
    let want = rule.process_name.trim().to_lowercase();
    if want.is_empty() {
        return false;
    }
    let exe = exe.to_lowercase();
    let bare = want.trim_end_matches(".exe").to_string();
    let name_ok = exe == want || exe == bare || exe == format!("{bare}.exe");
    if !name_ok {
        return false;
    }
    let needle = rule.window_title_contains.trim();
    needle.is_empty() || title.to_lowercase().contains(&needle.to_lowercase())
}

fn spawn_desktop_worker(app: tauri::AppHandle, shared: SharedState) {
    let usb = shared.lock().unwrap_or_else(|e| e.into_inner()).usb_bus.clone();
    std::thread::spawn(move || {
        let mut stable_exe = String::new();
        let mut stable_title = String::new();
        let mut stable_since = 0u64;
        let mut last_sample = 0u64;
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let now = now_ms();
            let should_sample = {
                let guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                (guard.sample_after_ms > 0 && now >= guard.sample_after_ms) || now.saturating_sub(last_sample) > 2000
            };
            if should_sample {
                last_sample = now;
                let changed = {
                    let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                    guard.sample_after_ms = 0;
                    sample_face(&mut guard)
                };
                if changed {
                    push_face_deltas(&shared, &usb, &app);
                } else {
                    let guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                    let _ = app.emit("deck-face", &guard.face);
                }
            }
            if let Some(window) = foreground_window() {
                let ignored = window.exe_name.eq_ignore_ascii_case("deckforge.exe");
                if ignored {
                    continue;
                }
                if window.exe_name != stable_exe || window.title != stable_title {
                    stable_exe = window.exe_name.clone();
                    stable_title = window.title.clone();
                    stable_since = now;
                    continue;
                }
                if now.saturating_sub(stable_since) < 800 {
                    continue;
                }
                let switch_to = {
                    let guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                    if now < guard.auto_switch_hold_until_ms {
                        None
                    } else {
                        let mut rules = guard.auto_switch_rules.clone();
                        rules.sort_by_key(|rule| rule.priority);
                        rules.into_iter().find(|rule| rule_matches(rule, &stable_exe, &stable_title)).and_then(|rule| {
                            if rule.target_profile_id == guard.active_profile_id {
                                None
                            } else if guard.profiles.iter().any(|profile| profile.id == rule.target_profile_id) {
                                Some(rule.target_profile_id)
                            } else {
                                None
                            }
                        })
                    }
                };
                if let Some(profile_id) = switch_to {
                    let event = {
                        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                        let name = guard
                            .profiles
                            .iter()
                            .find(|profile| profile.id == profile_id)
                            .map(|profile| profile.name.clone())
                            .unwrap_or_else(|| profile_id.clone());
                        guard.active_profile_id = profile_id.clone();
                        guard.active_page_index = 0;
                        DeckContextEvent {
                            profile_id,
                            page_index: 0,
                            message: format!("{stable_exe} is in front. Switched to {name}"),
                        }
                    };
                    push_active_layout(&shared, &usb, false);
                    let _ = app.emit("deck-context", &event);
                    stable_since = now_ms();
                }
            }
        }
    });
}

/// Spawns a dedicated background Rust thread that listens for `ACK\0\0OK` input reports
/// from the physical Ajazz AKP05E Pro (`VID_0300&PID_3013`, `MI_00`), emits `"hardware-input"`
/// events to the UI, and dispatches bound Win32 actions when buttons/knobs are pressed.
fn spawn_hardware_input_worker(app_handle: tauri::AppHandle, shared: SharedState) {
    let usb = shared
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .usb_bus
        .clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 2048];
        let mut held: Vec<(u8, u8, bool)> = Vec::new();
        let mut last_tap_ms: Vec<(u8, u8, u64)> = Vec::new();
        let mut strip_gesture = StripGesture::default();
        let mut seen_generation = 0u64;
        loop {
            let generation = deck_generation();
            if generation != seen_generation && generation > 0 {
                // The deck was opened (first start, or after a real unplug). Only
                // then do the pictures go out in full.
                seen_generation = generation;
                held.clear();
                last_tap_ms.clear();
                strip_gesture = StripGesture::default();
                push_connected_layout(&shared, &usb);
                continue;
            }
            let Some(raw) = deck_next_report(60) else {
                continue;
            };
            let n = raw.len().min(buf.len());
            buf[..n].copy_from_slice(&raw[..n]);
            {
                {
                        {
                            let ts = now_ms();
                            let Some(mut evt) = parse_input_report("ajazz-akp05e-pro", &buf[..n], ts) else {
                                continue;
                            };
                            if evt.kind == InputControlKind::TouchStripPoint {
                                let Some(resolved) = resolve_strip_point(&mut strip_gesture, &evt) else {
                                    continue;
                                };
                                evt = resolved;
                            }
                            if matches!(evt.kind, InputControlKind::Unknown) {
                                continue;
                            }
                            let is_turn = matches!(
                                evt.kind,
                                InputControlKind::RotaryKnobTurnCw | InputControlKind::RotaryKnobTurnCcw
                            );
                            let one_shot = matches!(
                                evt.kind,
                                InputControlKind::TouchStripTap
                                    | InputControlKind::TouchStripSwipe
                                    | InputControlKind::RotaryKnobPush
                            );
                            let slot = (kind_code(&evt.kind), evt.control_index);
                            if one_shot {
                                let prev = last_tap_ms
                                    .iter()
                                    .find(|(k, i, _)| *k == slot.0 && *i == slot.1)
                                    .map(|(_, _, t)| *t)
                                    .unwrap_or(0);
                                if let Some(entry) = last_tap_ms.iter_mut().find(|(k, i, _)| *k == slot.0 && *i == slot.1) {
                                    entry.2 = ts;
                                } else {
                                    last_tap_ms.push((slot.0, slot.1, ts));
                                }
                                // A press and its release arrive as two reports. A swipe can repeat.
                                let gap = if matches!(evt.kind, InputControlKind::TouchStripSwipe) {
                                    450
                                } else {
                                    280
                                };
                                if ts.saturating_sub(prev) < gap {
                                    continue;
                                }
                            } else if !is_turn {
                                let was = held
                                    .iter()
                                    .find(|(k, i, _)| *k == slot.0 && *i == slot.1)
                                    .map(|(_, _, down)| *down)
                                    .unwrap_or(false);
                                if let Some(entry) = held.iter_mut().find(|(k, i, _)| *k == slot.0 && *i == slot.1) {
                                    entry.2 = evt.pressed;
                                } else {
                                    held.push((slot.0, slot.1, evt.pressed));
                                }
                                if evt.pressed == was {
                                    continue;
                                }
                            }
                            let _ = app_handle.emit("hardware-input", &evt);
                            let pending = if evt.pressed || is_turn {
                                shared.lock().ok().and_then(|mut guard| {
                                    guard.recent_input_events.push(evt.clone());
                                    if guard.recent_input_events.len() > 25 {
                                        guard.recent_input_events.remove(0);
                                    }
                                    action_for_event(&guard, &evt)
                                })
                            } else {
                                None
                            };
                            if evt.pressed || is_turn {
                                if let Some(dev) = shared.lock().ok().and_then(|guard| {
                                    guard
                                        .devices
                                        .iter()
                                        .find(|d| d.connected && d.protocol_family.starts_with("CRT"))
                                        .cloned()
                                }) {
                                    let _bus = usb.lock().unwrap_or_else(|e| e.into_inner());
                                    let _ = wake_panel(&dev);
                                }
                            }
                            if let Some(action) = pending {
                                let message = dispatch_live_action(&app_handle, &shared, &usb, &action);
                                if let Ok(mut guard) = shared.lock() {
                                    guard.diagnostics.push(DiagnosticEntry {
                                        id: format!("diag-hw-{}", ts),
                                        timestamp_ms: ts,
                                        level: "success".to_string(),
                                        device_id: action.device_id,
                                        stage: "HW_KEY_PRESS".to_string(),
                                        message: format!("{} → {}", action.note, message),
                                        packet_hex: Some(evt.raw_hex.clone()),
                                    });
                                    if guard.diagnostics.len() > 80 {
                                        let extra = guard.diagnostics.len() - 80;
                                        guard.diagnostics.drain(0..extra);
                                    }
                                }
                            }
                        }
                }
            }
        }
    });
}

/// Cycles GIF frames onto key LCDs and the touch strip. At most three screens are
/// written per tick so the USB bus stays responsive.
fn spawn_animation_worker(shared: SharedState) {
    let usb = shared
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .usb_bus
        .clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(40));
        let due = {
            let Ok(mut guard) = shared.lock() else { continue };
            let now = now_ms();
            let dev = guard
                .devices
                .iter()
                .find(|d| d.id == guard.playback_device_id && d.connected)
                .cloned();
            let Some(dev) = dev else { continue };
            let mut pending = Vec::new();
            for slot in guard.playback_slots.iter_mut() {
                if pending.len() >= 3 || now < slot.next_due_ms || slot.jpegs.is_empty() {
                    continue;
                }
                let idx = slot.frame_index.min(slot.jpegs.len() - 1);
                let jpeg = slot.jpegs[idx].clone();
                let delay = slot.delays_ms.get(idx).copied().unwrap_or(100).max(80);
                let slot_id = slot.slot_id.clone();
                let target = slot.target.clone();
                if slot.frame_index + 1 >= slot.jpegs.len() {
                    if slot.loop_playback {
                        slot.frame_index = 0;
                    } else {
                        slot.frame_index = slot.jpegs.len() - 1;
                        slot.next_due_ms = u64::MAX;
                    }
                } else {
                    slot.frame_index += 1;
                }
                if slot.next_due_ms != u64::MAX {
                    slot.next_due_ms = now.saturating_add(delay);
                }
                pending.push((slot_id, target, jpeg));
            }
            let mut bursts = Vec::new();
            for (slot_id, target, jpeg) in pending {
                if guard
                    .frame_cache
                    .should_transmit(&dev.id, &slot_id, &jpeg)
                    .is_none()
                {
                    continue;
                }
                let reports = match target {
                    PlaybackTarget::Key(index) => {
                        build_key_image_packets(key_image_slot(&dev, index), &jpeg)
                    }
                    PlaybackTarget::TouchZone(index) => {
                        build_key_image_packets(touch_zone_image_slot(&dev, index), &jpeg)
                    }
                    PlaybackTarget::Strip { width, height } => {
                        build_touch_strip_packets(&jpeg, 0, 0, width, height)
                    }
                };
                bursts.push(HidBurst {
                    reports: reports.into_iter().map(|r| r.to_vec()).collect(),
                });
            }
            if bursts.is_empty() {
                None
            } else {
                Some((dev, bursts))
            }
        };
        if let Some((dev, bursts)) = due {
            let _bus = usb.lock().unwrap_or_else(|e| e.into_inner());
            let _ = write_animation_frames(&dev, &bursts);
        }
    });
}

#[tauri::command]
fn get_initial_state(state: State<'_, SharedState>) -> Result<FullBootstrapPayload, String> {
    let (mut devices, mut diagnostics, profiles, auto_switch_rules, macros, saved_locally_at_ms, applied_to_device_at_ms, recent_input_events, media_library, face, obs) = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        (
            guard.devices.clone(),
            guard.diagnostics.clone(),
            guard.profiles.clone(),
            guard.auto_switch_rules.clone(),
            guard.macros.clone(),
            guard.saved_locally_at_ms,
            guard.applied_to_device_at_ms,
            guard.recent_input_events.clone(),
            guard.media.previews(),
            guard.face.clone(),
            guard.obs.clone(),
        )
    };
    scan_usb_devices(&mut devices, &mut diagnostics);
    if diagnostics.len() > 80 {
        let extra = diagnostics.len() - 80;
        diagnostics.drain(0..extra);
    }
    let telemetry = collect_system_telemetry();
    let legacy_summary = scan_legacy_streamdock_environment();
    let frame_cache_stats = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        guard.devices = devices.clone();
        guard.diagnostics = diagnostics.clone();
        get_cache_stats(&guard.frame_cache)
    };

    Ok(FullBootstrapPayload {
        devices,
        profiles,
        auto_switch_rules,
        macros,
        diagnostics,
        frame_cache_stats,
        legacy_summary,
        telemetry,
        saved_locally_at_ms,
        applied_to_device_at_ms,
        recent_input_events,
        media_library,
        face,
        obs,
    })
}

#[tauri::command]
fn refresh_usb_devices(state: State<'_, SharedState>) -> Result<Vec<DeviceDescriptor>, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let mut devices = guard.devices.clone();
    let mut diags = guard.diagnostics.clone();
    scan_usb_devices(&mut devices, &mut diags);
    guard.devices = devices.clone();
    guard.diagnostics = diags;
    Ok(devices)
}

#[tauri::command]
fn set_device_brightness(
    device_id: String,
    percent: u8,
    state: State<'_, SharedState>,
) -> Result<String, String> {
    let percent = percent.min(100);
    let (dev, usb) = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        let Some(dev) = guard.devices.iter_mut().find(|d| d.id == device_id) else {
            return Err(format!("Device {} not found", device_id));
        };
        dev.brightness = percent;
        let dev = dev.clone();
        let usb = guard.usb_bus.clone();
        (dev, usb)
    };
    let _bus = usb.lock().unwrap_or_else(|e| e.into_inner());
    let mut diags = Vec::new();
    let res = send_hardware_brightness(&dev, percent, &mut diags)?;
    drop(_bus);
    if let Ok(mut guard) = state.lock() {
        guard.diagnostics.extend(diags);
    }
    Ok(res)
}

#[tauri::command]
fn apply_profile_to_device(
    device_id: String,
    profile_id: String,
    page_id: String,
    state: State<'_, SharedState>,
) -> Result<SyncKeyResult, String> {
    let usb = state
        .lock()
        .map_err(|e| e.to_string())?
        .usb_bus
        .clone();

    let (dev, bursts, result) = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        guard.active_profile_id = profile_id.clone();

        let dev = guard
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .cloned()
            .ok_or_else(|| format!("Device {} not found", device_id))?;

        let prof = guard
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .ok_or_else(|| format!("Profile {} not found", profile_id))?;

        let page_index = prof
            .pages
            .iter()
            .position(|pg| pg.id == page_id)
            .unwrap_or(0);
        guard.active_page_index = page_index;
        let page = prof
            .pages
            .get(page_index)
            .or_else(|| prof.pages.first())
            .cloned()
            .ok_or_else(|| "Profile has no pages".to_string())?;

        guard.frame_cache.invalidate_device(&dev.id);
        let (jobs, zones, strip) = plan_screens(&dev, &page, &guard.face);

        let mut diags = guard.diagnostics.clone();
        let media = std::mem::take(&mut guard.media);
        let outcome = publish_screens(
            &dev,
            &jobs,
            &zones,
            strip.as_ref(),
            &media,
            &mut guard.frame_cache,
            &mut diags,
        );
        guard.media = media;
        guard.diagnostics = diags;
        guard.playback_slots = outcome.playback;
        guard.playback_device_id = dev.id.clone();
        guard.applied_to_device_at_ms = Some(now_ms());
        (dev, outcome.bursts, outcome.result)
    };

    {
        let _bus = usb.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(err) = write_hid_bursts(&dev, &bursts) {
            return Err(err);
        }
    }
    Ok(result)
}

#[tauri::command]
fn clear_device_key(
    device_id: String,
    key_index_1based: u8,
    state: State<'_, SharedState>,
) -> Result<String, String> {
    let (dev, packet, usb, message) = {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        if key_index_1based == 0xFF {
            guard.playback_slots.clear();
        } else {
            let idx = key_index_1based.saturating_sub(1);
            guard.playback_slots.retain(|slot| !matches!(slot.target, PlaybackTarget::Key(i) if i == idx));
        }
        let dev = guard
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .cloned()
            .ok_or_else(|| format!("Device {} not found", device_id))?;
        let mut diags = guard.diagnostics.clone();
        let (message, packet) =
            clear_hardware_key(&dev, key_index_1based, &mut guard.frame_cache, &mut diags)?;
        guard.diagnostics = diags;
        (dev, packet, guard.usb_bus.clone(), message)
    };
    let _bus = usb.lock().unwrap_or_else(|e| e.into_inner());
    let _ = write_one_report(&dev, &packet);
    Ok(message)
}

#[tauri::command]
fn save_profiles(profiles: Vec<Profile>, state: State<'_, SharedState>) -> Result<u64, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let ts = now_ms();
    write_saved_profiles(&profiles)?;
    guard.profiles = profiles;
    guard.saved_locally_at_ms = ts;
    Ok(ts)
}

#[tauri::command]
fn test_key_action(
    action_id: String,
    action_label: String,
    action_param: String,
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
) -> Result<ActionExecutionResult, String> {
    let usb = state.lock().map_err(|e| e.to_string())?.usb_bus.clone();
    let action = LiveAction {
        action_id: action_id.clone(),
        label: action_label.clone(),
        param: action_param.clone(),
        note: "Test".to_string(),
        device_id: "action-runner".to_string(),
    };
    let message = dispatch_live_action(&app, state.inner(), &usb, &action);
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let res = ActionExecutionResult {
        action_id,
        action_label,
        success: !message.contains("failed") && !message.contains("not connected"),
        elapsed_ms: 1,
        output_message: message,
        timestamp_ms: now_ms(),
    };
    guard.diagnostics.push(DiagnosticEntry {
        id: format!("diag-act-{}", res.timestamp_ms),
        timestamp_ms: res.timestamp_ms,
        level: "info".to_string(),
        device_id: "action-runner".to_string(),
        stage: "WIN32_ACTION".to_string(),
        message: res.output_message.clone(),
        packet_hex: None,
    });
    Ok(res)
}

fn find_macro(guard: &ManagedAppState, macro_id: &str) -> Option<db::MacroSequence> {
    let needle = macro_id.trim();
    guard.macros.iter().find(|m| m.id == needle || m.name.eq_ignore_ascii_case(needle)).cloned()
}

fn run_one_macro_step(
    app: &tauri::AppHandle,
    shared: &SharedState,
    usb: &Arc<Mutex<()>>,
    step: &db::MacroStep,
) -> (bool, String) {
    match step.step_type.as_str() {
        "Delay" => {
            let ms = macro_delay_ms(&step.parameter);
            std::thread::sleep(std::time::Duration::from_millis(ms));
            (true, format!("Waited {ms}ms"))
        }
        "Shortcut" => match send_hotkey_chord(&step.parameter) {
            Ok(msg) => (true, msg),
            Err(err) => (false, err),
        },
        "LaunchApp" => match launch_app_or_url(&step.parameter) {
            Ok(msg) => (true, msg),
            Err(err) => (false, err),
        },
        "HttpRequest" => match trigger_http_webhook(&step.parameter) {
            Ok(msg) => (true, msg),
            Err(err) => (false, err),
        },
        "IfWindowExists" => {
            let running = process_is_running(&step.parameter);
            if running {
                (true, format!("{} is running", step.parameter))
            } else {
                (false, format!("{} is not running", step.parameter))
            }
        }
        "SwitchProfile" => match apply_navigation(app, shared, usb, "profile.switch", &step.parameter) {
            Some(msg) => (true, msg),
            None => (false, format!("No profile named {}", step.parameter)),
        },
        other => (false, format!("Unknown step {other}")),
    }
}

fn run_macro_sequence(
    app: &tauri::AppHandle,
    shared: &SharedState,
    usb: &Arc<Mutex<()>>,
    seq: &db::MacroSequence,
) -> MacroRunReport {
    use actions::MacroStepResult;
    let start = std::time::Instant::now();
    let mut step_results = Vec::with_capacity(seq.steps.len());
    for step in &seq.steps {
        let started = std::time::Instant::now();
        let tries = if step.on_error == "RetryOnce" { 2 } else { 1 };
        let mut outcome = (false, String::new());
        for _ in 0..tries {
            outcome = run_one_macro_step(app, shared, usb, step);
            if outcome.0 {
                break;
            }
        }
        let passed = outcome.0;
        step_results.push(MacroStepResult {
            step_id: step.id.clone(),
            label: step.label.clone(),
            step_type: step.step_type.clone(),
            status: if passed { "Passed" } else { "Failed" }.to_string(),
            elapsed_ms: started.elapsed().as_millis().max(1) as u64,
            detail: outcome.1,
        });
        if !passed && step.on_error != "Continue" {
            break;
        }
    }
    MacroRunReport {
        macro_id: seq.id.clone(),
        macro_name: seq.name.clone(),
        total_elapsed_ms: start.elapsed().as_millis().max(1) as u64,
        step_results,
    }
}

fn start_macro(
    app: &tauri::AppHandle,
    shared: &SharedState,
    usb: &Arc<Mutex<()>>,
    macro_id: &str,
    label: &str,
) -> String {
    let seq = shared.lock().ok().and_then(|guard| find_macro(&guard, macro_id));
    let Some(seq) = seq else {
        return format!("No macro named {macro_id}");
    };
    let name = seq.name.clone();
    let app = app.clone();
    let shared = shared.clone();
    let usb = usb.clone();
    std::thread::spawn(move || {
        let report = run_macro_sequence(&app, &shared, &usb, &seq);
        if let Ok(mut guard) = shared.lock() {
            guard.diagnostics.push(DiagnosticEntry {
                id: format!("diag-macro-{}", now_ms()),
                timestamp_ms: now_ms(),
                level: if report.step_results.iter().any(|step| step.status != "Passed") {
                    "warning"
                } else {
                    "success"
                }
                .to_string(),
                device_id: "macro-runner".to_string(),
                stage: "MACRO_RUN".to_string(),
                message: format!(
                    "Ran {} ({} steps, {}ms)",
                    report.macro_name,
                    report.step_results.len(),
                    report.total_elapsed_ms
                ),
                packet_hex: None,
            });
        }
        let _ = app.emit("macro-finished", &report);
    });
    if label.is_empty() {
        format!("Started {name}")
    } else {
        format!("Started {label}")
    }
}

#[tauri::command]
fn save_macros(macros: Vec<db::MacroSequence>, state: State<'_, SharedState>) -> Result<(), String> {
    write_saved_macros(&macros)?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.macros = macros;
    Ok(())
}

#[tauri::command]
fn execute_macro_sequence(
    macro_id: String,
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
) -> Result<MacroRunReport, String> {
    let usb = state.lock().map_err(|e| e.to_string())?.usb_bus.clone();
    let seq = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        find_macro(&guard, &macro_id).ok_or_else(|| format!("Macro {} not found", macro_id))?
    };
    let report = run_macro_sequence(&app, state.inner(), &usb, &seq);
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.diagnostics.push(DiagnosticEntry {
        id: format!("diag-macro-{}", now_ms()),
        timestamp_ms: now_ms(),
        level: "success".to_string(),
        device_id: "macro-runner".to_string(),
        stage: "MACRO_TEST".to_string(),
        message: format!(
            "Tested macro `{}` ({} steps completed in {}ms)",
            report.macro_name,
            report.step_results.len(),
            report.total_elapsed_ms
        ),
        packet_hex: None,
    });
    Ok(report)
}

#[tauri::command]
fn take_over_from_official_app(state: State<'_, SharedState>) -> Result<String, String> {
    let msg = stop_official_streamdock_app()?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let mut devices = guard.devices.clone();
    let mut diags = guard.diagnostics.clone();
    scan_usb_devices(&mut devices, &mut diags);
    diags.push(DiagnosticEntry {
        id: format!("diag-takeover-{}", now_ms()),
        timestamp_ms: now_ms(),
        level: "success".to_string(),
        device_id: "ajazz-akp05e-pro".to_string(),
        stage: "EXCLUSIVE_HANDOFF".to_string(),
        message: msg.clone(),
        packet_hex: Some("CRT\\0\\0HAN".to_string()),
    });
    guard.devices = devices;
    guard.diagnostics = diags;
    Ok(msg)
}

#[tauri::command]
fn import_legacy_sdprofiles(state: State<'_, SharedState>) -> Result<Vec<Profile>, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let imported = parse_all_legacy_sdprofiles();
    let mut count = 0usize;
    for imp in imported {
        if !guard.profiles.iter().any(|p| p.id == imp.id) {
            guard.profiles.push(imp);
            count += 1;
        }
    }
    guard.diagnostics.push(DiagnosticEntry {
        id: format!("diag-import-{}", now_ms()),
        timestamp_ms: now_ms(),
        level: "success".to_string(),
        device_id: "profile-store".to_string(),
        stage: "SDPROFILE_IMPORT".to_string(),
        message: format!(
            "Parsed & imported {} legacy .sdProfile configurations with Win32 hotkey mappings",
            count
        ),
        packet_hex: None,
    });
    Ok(guard.profiles.clone())
}

#[tauri::command]
fn get_telemetry() -> SystemTelemetrySnapshot {
    collect_system_telemetry()
}

#[tauri::command]
fn set_active_context(
    profile_id: String,
    page_index: usize,
    state: State<'_, SharedState>,
) -> Result<(), String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    if guard.profiles.iter().any(|p| p.id == profile_id) {
        guard.active_profile_id = profile_id;
    }
    guard.active_page_index = page_index;
    Ok(())
}

#[tauri::command]
fn hold_auto_switch(state: State<'_, SharedState>) -> Result<(), String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.auto_switch_hold_until_ms = now_ms().saturating_add(6000);
    Ok(())
}

#[tauri::command]
fn save_auto_switch_rules(rules: Vec<AutoSwitchRule>, state: State<'_, SharedState>) -> Result<(), String> {
    write_auto_switch_rules(&rules)?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.auto_switch_rules = rules;
    Ok(())
}

#[tauri::command]
fn save_obs_settings(settings: ObsSettings, state: State<'_, SharedState>) -> Result<(), String> {
    settings.save()?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.obs = settings;
    Ok(())
}

#[tauri::command]
fn stage_screen_media(
    id: String,
    file_name: String,
    data_base64: String,
    state: State<'_, SharedState>,
) -> Result<MediaPreview, String> {
    let bytes = decode_b64(&data_base64)?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.media.stage_bytes(&id, &file_name, &bytes)
}

#[tauri::command]
fn stage_frame_sequence(
    id: String,
    file_name: String,
    frames_base64: Vec<String>,
    delay_ms: u32,
    state: State<'_, SharedState>,
) -> Result<MediaPreview, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard
        .media
        .stage_sequence(&id, &file_name, &frames_base64, delay_ms)
}

#[tauri::command]
async fn game_mode_scan() -> Result<game_mode::ScanReport, String> {
    tauri::async_runtime::spawn_blocking(game_mode::scan)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn game_mode_get_config() -> game_mode::GameModeConfig {
    game_mode::config()
}

#[tauri::command]
fn game_mode_save_config(config: game_mode::GameModeConfig) -> Result<game_mode::GameModeConfig, String> {
    game_mode::save_config(config)
}

#[tauri::command]
fn game_mode_status() -> game_mode::GameModeStatus {
    game_mode::status()
}

/// Turns Game Mode on or off. `enabled: None` flips it. Work happens on a thread and the
/// result arrives through the `game-mode` event.
#[tauri::command]
fn game_mode_set(
    enabled: Option<bool>,
    mode: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
) -> Result<String, String> {
    if game_mode::is_busy() {
        return Err("Game Mode is still working.".to_string());
    }
    let want = enabled.unwrap_or(!game_mode::is_active());
    if want == game_mode::is_active() {
        return Ok(if want { "Game Mode is already on." } else { "Game Mode is already off." }.to_string());
    }
    let shared = Arc::clone(state.inner());
    std::thread::spawn(move || {
        let message = if want {
            game_mode::enable(&app, false, None, mode.as_deref().filter(|m| !m.is_empty()))
        } else {
            game_mode::disable(&app, false)
        };
        nudge_face_sample(&shared);
        let _ = app.emit("game-mode-message", &message);
    });
    nudge_face_sample(state.inner());
    Ok(if want { "Starting Game Mode" } else { "Ending Game Mode" }.to_string())
}

/// Starts the admin helper while the user is at the desktop, so Game Mode can stop services
/// later without a Windows prompt. Refuses while something is full-screen.
#[tauri::command]
fn game_mode_arm_helper(app: tauri::AppHandle) -> Result<String, String> {
    if game_mode::is_busy() {
        return Err("Game Mode is still working.".to_string());
    }
    std::thread::spawn(move || {
        let message = game_mode::arm(&app);
        let _ = app.emit("game-mode-message", &message);
    });
    Ok("Starting the admin helper".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// The official Stream Dock app starts with Windows, redraws the deck every second and,
/// when it exits, tells the deck to disconnect, which blanks it. With both programs on the
/// same deck neither gets a stable picture. This stops the official app whenever it shows
/// up and sends the layout again afterwards. Creating `%APPDATA%\DeckForge\allow-official-app`
/// turns this off for people who want to run both.
fn spawn_official_app_guard() {
    std::thread::spawn(|| {
        let marker = std::env::var_os("APPDATA")
            .map(|dir| std::path::PathBuf::from(dir).join("DeckForge").join("allow-official-app"));
        std::thread::sleep(Duration::from_secs(1));
        loop {
            let allowed = marker.as_ref().map(|p| p.exists()).unwrap_or(false);
            if !allowed {
                let stopped = actions::kill_official_streamdock();
                if !stopped.is_empty() {
                    deck_note(&format!(
                        "official app stopped: {} (create allow-official-app to keep it)",
                        stopped.join(", ")
                    ));
                    std::thread::sleep(Duration::from_millis(1500));
                    request_full_push("official app was running");
                }
            }
            std::thread::sleep(Duration::from_secs(5));
        }
    });
}

pub fn run() {
    let shared: SharedState = Arc::new(Mutex::new(ManagedAppState::default()));
    let shared_for_setup = Arc::clone(&shared);

    tauri::Builder::default()
        .manage(shared)
        .setup(move |app| {
            let handle = app.handle().clone();
            spawn_hardware_input_worker(handle.clone(), Arc::clone(&shared_for_setup));
            spawn_official_app_guard();
            spawn_animation_worker(Arc::clone(&shared_for_setup));
            let nudge_shared = Arc::clone(&shared_for_setup);
            game_mode::spawn_watcher(
                handle.clone(),
                Arc::new(move || nudge_face_sample(&nudge_shared)),
            );
            spawn_desktop_worker(handle, Arc::clone(&shared_for_setup));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_initial_state,
            refresh_usb_devices,
            set_device_brightness,
            apply_profile_to_device,
            clear_device_key,
            save_profiles,
            save_macros,
            test_key_action,
            execute_macro_sequence,
            take_over_from_official_app,
            import_legacy_sdprofiles,
            get_telemetry,
            set_active_context,
            hold_auto_switch,
            save_auto_switch_rules,
            save_obs_settings,
            stage_screen_media,
            stage_frame_sequence,
            game_mode_scan,
            game_mode_get_config,
            game_mode_save_config,
            game_mode_status,
            game_mode_set,
            game_mode_arm_helper
        ])
        .run(tauri::generate_context!())
        .expect("error while running DeckForge Tauri application");
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_live_usb_and_legacy_streamdock_discovery() {
        let state = ManagedAppState::default();
        let legacy = scan_legacy_streamdock_environment();
        let imported = parse_all_legacy_sdprofiles();
        let telemetry = collect_system_telemetry();

        println!(
            "[DeckForge Integration] Legacy profiles found: {} ({} rich profiles parsed), Legacy plugins found: {}",
            legacy.legacy_profile_count,
            imported.len(),
            legacy.legacy_plugin_count
        );
        for p in imported.iter().take(5) {
            let configured_keys = p.pages[0]
                .keys
                .iter()
                .filter(|k| k.action_id != "none")
                .map(|k| format!("{}:{}", k.title, k.action_param))
                .collect::<Vec<_>>();
            println!(
                "[DeckForge Imported Profile] {} -> {:?}",
                p.name, configured_keys
            );
        }
        for d in &state.devices {
            println!(
                "[DeckForge Integration] Device: {} ({}) connected={} serial={} path={:?}",
                d.model_name, d.vid_pid_hex, d.connected, d.serial_number, d.hid_path
            );
        }
        println!(
            "[DeckForge Integration] System CPU: {}%, RAM: {}/{} GB, Official StreamDock.exe running: {}",
            telemetry.cpu_usage_percent,
            telemetry.memory_used_gb,
            telemetry.memory_total_gb,
            telemetry.official_streamdock_running
        );

        assert!(!state.devices.is_empty());
        assert!(!state.profiles.is_empty());
        assert!(!imported.is_empty());
    }

    #[test]
    fn test_live_hardware_handshake_brightness_and_frame_sync() {
        let mut state = ManagedAppState::default();
        let akp = state
            .devices
            .iter()
            .find(|d| d.id == "ajazz-akp05e-pro")
            .cloned()
            .expect("Ajazz AKP05E Pro descriptor exists");

        println!(
            "[Live E2E Test] Target Device: {} ({}) connected={} serial={}",
            akp.model_name, akp.vid_pid_hex, akp.connected, akp.serial_number
        );

        // 1. Test live hardware brightness command (CRT\0\0LIG -> 90%)
        let mut diags = state.diagnostics.clone();
        let bright_res = send_hardware_brightness(&akp, 90, &mut diags);
        println!("[Live E2E Test] Brightness Result: {:?}", bright_res);
        assert!(bright_res.is_ok());

        // 2. Test Pass 1 of Profile Key Sync (10 keys -> transmits 10 keys + 4-zone touch strip over USB)
        state.frame_cache = AntiFreezeFrameCache::new();
        let page = &state.profiles[0].pages[0];
        let payload: Vec<(u8, String, String, String, String)> = page
            .keys
            .iter()
            .take(10)
            .map(|k| {
                (
                    k.index,
                    k.title.clone(),
                    k.icon_name.clone(),
                    k.accent_color.clone(),
                    k.bg_color.clone(),
                )
            })
            .collect();
        let touch_payload: Vec<(String, String, String)> = page
            .touch_zones
            .iter()
            .map(|z| (z.label.clone(), z.value_text.clone(), z.accent_color.clone()))
            .collect();

        let sync_pass_1 = sync_keys_to_hardware(
            &akp,
            &payload,
            &touch_payload,
            &mut state.frame_cache,
            &mut diags,
        );
        println!(
            "[Live E2E Test] Sync Pass #1 (Initial): physical={} transmitted={}/{} skipped={} bytes={} elapsed={}ms",
            sync_pass_1.connected_physical,
            sync_pass_1.keys_transmitted,
            sync_pass_1.keys_evaluated,
            sync_pass_1.keys_deduplicated_skipped,
            sync_pass_1.bytes_transmitted,
            sync_pass_1.elapsed_ms
        );
        assert_eq!(sync_pass_1.keys_transmitted, 10);
        assert_eq!(sync_pass_1.keys_deduplicated_skipped, 0);

        // 3. Test Pass 2 with identical keys -> Anti-Freeze SHA-256 Cache MUST skip all 10 keys (0 USB bytes!)
        let sync_pass_2 = sync_keys_to_hardware(
            &akp,
            &payload,
            &touch_payload,
            &mut state.frame_cache,
            &mut diags,
        );
        println!(
            "[Live E2E Test] Sync Pass #2 (Unchanged Redraw): physical={} transmitted={}/{} skipped={} bytes={} elapsed={}ms",
            sync_pass_2.connected_physical,
            sync_pass_2.keys_transmitted,
            sync_pass_2.keys_evaluated,
            sync_pass_2.keys_deduplicated_skipped,
            sync_pass_2.bytes_transmitted,
            sync_pass_2.elapsed_ms
        );
        assert_eq!(sync_pass_2.keys_transmitted, 0);
        assert_eq!(sync_pass_2.keys_deduplicated_skipped, 10);
        assert_eq!(sync_pass_2.bytes_transmitted, 0);

        // 4. Test Pass 3 after modifying ONLY Key #1 -> ONLY 1 key is transmitted over USB!
        let mut modified_payload = payload.clone();
        modified_payload[0].1 = "Start Stream".to_string();
        modified_payload[0].3 = "#EC4899".to_string();
        let sync_pass_3 = sync_keys_to_hardware(
            &akp,
            &modified_payload,
            &touch_payload,
            &mut state.frame_cache,
            &mut diags,
        );
        println!(
            "[Live E2E Test] Sync Pass #3 (1 Key Edited): physical={} transmitted={}/{} skipped={} bytes={} elapsed={}ms",
            sync_pass_3.connected_physical,
            sync_pass_3.keys_transmitted,
            sync_pass_3.keys_evaluated,
            sync_pass_3.keys_deduplicated_skipped,
            sync_pass_3.bytes_transmitted,
            sync_pass_3.elapsed_ms
        );
        assert_eq!(sync_pass_3.keys_transmitted, 1);
        assert_eq!(sync_pass_3.keys_deduplicated_skipped, 9);

        // Restore original Key #1 ("Start Stream" #EF4444) so the physical device matches the UI screenshot
        let _ = sync_keys_to_hardware(
            &akp,
            &payload,
            &touch_payload,
            &mut state.frame_cache,
            &mut diags,
        );
    }
}
