pub mod actions;
pub mod db;
pub mod drivers;

use actions::{
    collect_system_telemetry, execute_key_action_live, run_macro_sequence_test,
    stop_official_streamdock_app, ActionExecutionResult, MacroRunReport, SystemTelemetrySnapshot,
};
use db::{
    default_auto_switch_rules, default_macros, default_profiles, parse_all_legacy_sdprofiles,
    scan_legacy_streamdock_environment, AutoSwitchRule, LegacyStreamDockSummary, MacroSequence,
    Profile,
};
use drivers::akp05e_protocol::{
    build_handshake_packet, parse_input_report, HardwareInputEvent, InputControlKind,
};
use drivers::frame_cache::{AntiFreezeFrameCache, FrameCacheStats};
use drivers::hal::{
    builtin_device_catalog, clear_hardware_key, get_cache_stats, now_ms, scan_usb_devices,
    send_hardware_brightness, sync_keys_to_hardware, DeviceDescriptor, DiagnosticEntry,
    SyncKeyResult,
};
use hidapi::HidApi;
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

        let profiles = default_profiles();
        let mut frame_cache = AntiFreezeFrameCache::new();
        let mut applied_to_device_at_ms = None;

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

                if let Some(prof) = profiles.first() {
                    if let Some(page) = prof.pages.first() {
                        let key_count = d.keys.count as usize;
                        let payload: Vec<(u8, String, String, String, String)> = page
                            .keys
                            .iter()
                            .take(key_count)
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
                        let _ = sync_keys_to_hardware(
                            d,
                            &payload,
                            &touch_payload,
                            &mut frame_cache,
                            &mut diagnostics,
                        );
                        applied_to_device_at_ms = Some(now_ms());
                    }
                }
            }
        }

        Self {
            devices,
            profiles,
            active_profile_id: "profile-streaming".to_string(),
            active_page_index: 0,
            auto_switch_rules: default_auto_switch_rules(),
            macros: default_macros(),
            diagnostics,
            recent_input_events: Vec::new(),
            frame_cache,
            saved_locally_at_ms: now_ms(),
            applied_to_device_at_ms,
        }
    }
}

pub type SharedState = Arc<Mutex<ManagedAppState>>;

/// Spawns a dedicated background Rust thread that listens for `ACK\0\0OK` input reports
/// from the physical Ajazz AKP05E Pro (`VID_0300&PID_3013`, `MI_00`), emits `"hardware-input"`
/// events to the UI, and dispatches bound Win32 actions when buttons/knobs are pressed.
fn spawn_hardware_input_worker(app_handle: tauri::AppHandle, shared: SharedState) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 513];
        loop {
            let api_opt = HidApi::new().ok();
            let mut handle_opt = None;

            if let Some(ref api) = api_opt {
                for info in api.device_list() {
                    if info.vendor_id() == 0x0300 && info.product_id() == 0x3013 {
                        let p = info.path().to_string_lossy().to_lowercase();
                        if p.contains("mi_00") || info.interface_number() == 0 {
                            if let Ok(h) = info.open_device(api) {
                                let _ = h.write(&build_handshake_packet());
                                handle_opt = Some(h);
                                break;
                            }
                        }
                    }
                }
            }

            if let Some(handle) = handle_opt {
                loop {
                    match handle.read_timeout(&mut buf, 60) {
                        Ok(n) if n > 0 => {
                            let ts = now_ms();
                            if let Some(evt) = parse_input_report("ajazz-akp05e-pro", &buf[..n], ts) {
                                let _ = app_handle.emit("hardware-input", &evt);

                                if let Ok(mut guard) = shared.lock() {
                                    guard.recent_input_events.push(evt.clone());
                                    if guard.recent_input_events.len() > 25 {
                                        guard.recent_input_events.remove(0);
                                    }

                                    // Dispatch live Win32 action on press or rotary turn
                                    if evt.pressed {
                                        let active_prof_id = guard.active_profile_id.clone();
                                        let page_idx = guard.active_page_index;
                                        if let Some(prof) = guard
                                            .profiles
                                            .iter()
                                            .find(|p| p.id == active_prof_id)
                                            .or_else(|| guard.profiles.first())
                                            .cloned()
                                        {
                                            if let Some(page) = prof
                                                .pages
                                                .get(page_idx)
                                                .or_else(|| prof.pages.first())
                                            {
                                                match evt.kind {
                                                    InputControlKind::LcdKey => {
                                                        if let Some(k) = page.keys.get(evt.control_index as usize) {
                                                            let res = execute_key_action_live(
                                                                &k.action_id,
                                                                &k.action_label,
                                                                &k.action_param,
                                                                true,
                                                            );
                                                            guard.diagnostics.push(DiagnosticEntry {
                                                                id: format!("diag-hw-{}", ts),
                                                                timestamp_ms: ts,
                                                                level: "success".to_string(),
                                                                device_id: "ajazz-akp05e-pro".to_string(),
                                                                stage: "HW_KEY_PRESS".to_string(),
                                                                message: format!(
                                                                    "Key #{} ({}) → {}",
                                                                    evt.control_index + 1,
                                                                    k.title,
                                                                    res.output_message
                                                                ),
                                                                packet_hex: Some(evt.raw_hex.clone()),
                                                            });
                                                        }
                                                    }
                                                    InputControlKind::RotaryKnobTurnCw => {
                                                        let param = page
                                                            .encoders
                                                            .get(evt.control_index as usize)
                                                            .map(|e| e.cw_action.as_str())
                                                            .unwrap_or("VK_VOLUME_UP");
                                                        let _ = execute_key_action_live(
                                                            "custom.hotkey",
                                                            "Knob CW Action",
                                                            param,
                                                            true,
                                                        );
                                                    }
                                                    InputControlKind::RotaryKnobTurnCcw => {
                                                        let param = page
                                                            .encoders
                                                            .get(evt.control_index as usize)
                                                            .map(|e| e.ccw_action.as_str())
                                                            .unwrap_or("VK_VOLUME_DOWN");
                                                        let _ = execute_key_action_live(
                                                            "custom.hotkey",
                                                            "Knob CCW Action",
                                                            param,
                                                            true,
                                                        );
                                                    }
                                                    InputControlKind::RotaryKnobPush => {
                                                        let param = page
                                                            .encoders
                                                            .get(evt.control_index as usize)
                                                            .map(|e| e.push_action.as_str())
                                                            .unwrap_or("VK_VOLUME_MUTE");
                                                        let _ = execute_key_action_live(
                                                            "custom.hotkey",
                                                            "Knob Push Action",
                                                            param,
                                                            true,
                                                        );
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(_) => {
                            // Device disconnected or handle invalidated; break inner loop and retry after backoff
                            break;
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(1500));
        }
    });
}

#[tauri::command]
fn get_initial_state(state: State<'_, SharedState>) -> Result<FullBootstrapPayload, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let mut devices = guard.devices.clone();
    let mut diags = guard.diagnostics.clone();
    scan_usb_devices(&mut devices, &mut diags);
    guard.devices = devices.clone();
    guard.diagnostics = diags.clone();

    Ok(FullBootstrapPayload {
        devices,
        profiles: guard.profiles.clone(),
        auto_switch_rules: guard.auto_switch_rules.clone(),
        macros: guard.macros.clone(),
        diagnostics: guard.diagnostics.clone(),
        frame_cache_stats: get_cache_stats(&guard.frame_cache),
        legacy_summary: scan_legacy_streamdock_environment(),
        telemetry: collect_system_telemetry(),
        saved_locally_at_ms: guard.saved_locally_at_ms,
        applied_to_device_at_ms: guard.applied_to_device_at_ms,
        recent_input_events: guard.recent_input_events.clone(),
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
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let dev_opt = guard.devices.iter_mut().find(|d| d.id == device_id).cloned();
    let Some(dev) = dev_opt else {
        return Err(format!("Device {} not found", device_id));
    };

    let mut diags = guard.diagnostics.clone();
    let res = send_hardware_brightness(&dev, percent, &mut diags)?;
    if let Some(d) = guard.devices.iter_mut().find(|d| d.id == device_id) {
        d.brightness = percent.min(100);
    }
    guard.diagnostics = diags;
    Ok(res)
}

#[tauri::command]
fn apply_profile_to_device(
    device_id: String,
    profile_id: String,
    page_id: String,
    state: State<'_, SharedState>,
) -> Result<SyncKeyResult, String> {
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

    let page = prof
        .pages
        .iter()
        .find(|pg| pg.id == page_id)
        .or_else(|| prof.pages.first())
        .cloned()
        .ok_or_else(|| "Profile has no pages".to_string())?;

    let key_count = dev.keys.count as usize;
    let payload: Vec<(u8, String, String, String, String)> = page
        .keys
        .iter()
        .take(key_count)
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

    let mut diags = guard.diagnostics.clone();
    let result = sync_keys_to_hardware(
        &dev,
        &payload,
        &touch_payload,
        &mut guard.frame_cache,
        &mut diags,
    );
    guard.diagnostics = diags;
    guard.applied_to_device_at_ms = Some(now_ms());

    Ok(result)
}

#[tauri::command]
fn clear_device_key(
    device_id: String,
    key_index_1based: u8,
    state: State<'_, SharedState>,
) -> Result<String, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let dev = guard
        .devices
        .iter()
        .find(|d| d.id == device_id)
        .cloned()
        .ok_or_else(|| format!("Device {} not found", device_id))?;

    let mut diags = guard.diagnostics.clone();
    let res = clear_hardware_key(&dev, key_index_1based, &mut guard.frame_cache, &mut diags)?;
    guard.diagnostics = diags;
    Ok(res)
}

#[tauri::command]
fn save_profiles(profiles: Vec<Profile>, state: State<'_, SharedState>) -> Result<u64, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let ts = now_ms();
    guard.profiles = profiles;
    guard.saved_locally_at_ms = ts;
    Ok(ts)
}

#[tauri::command]
fn test_key_action(
    action_id: String,
    action_label: String,
    action_param: String,
    state: State<'_, SharedState>,
) -> Result<ActionExecutionResult, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let res = execute_key_action_live(&action_id, &action_label, &action_param, true);
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

#[tauri::command]
fn execute_macro_sequence(
    macro_id: String,
    state: State<'_, SharedState>,
) -> Result<MacroRunReport, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let seq = guard
        .macros
        .iter()
        .find(|m| m.id == macro_id)
        .cloned()
        .ok_or_else(|| format!("Macro {} not found", macro_id))?;

    let report = run_macro_sequence_test(&seq, false);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared: SharedState = Arc::new(Mutex::new(ManagedAppState::default()));
    let shared_for_setup = Arc::clone(&shared);

    tauri::Builder::default()
        .manage(shared)
        .setup(move |app| {
            let handle = app.handle().clone();
            spawn_hardware_input_worker(handle, Arc::clone(&shared_for_setup));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_initial_state,
            refresh_usb_devices,
            set_device_brightness,
            apply_profile_to_device,
            clear_device_key,
            save_profiles,
            test_key_action,
            execute_macro_sequence,
            take_over_from_official_app,
            import_legacy_sdprofiles,
            get_telemetry
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
