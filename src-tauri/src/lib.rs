pub mod actions;
pub mod db;
pub mod drivers;
pub mod game_mode;
pub mod media;
mod media_download;
pub mod plugins;
pub mod updates;

use actions::{
    collect_system_telemetry, foreground_window, launch_app_or_url,
    macro_delay_ms, process_is_running, send_hotkey_chord,
    ActionExecutionResult, MacroRunReport,
    ObsSettings, SystemTelemetrySnapshot,
};
use db::{
    load_auto_switch_rules, load_saved_macros, load_saved_profiles, parse_all_legacy_sdprofiles,
    scan_legacy_streamdock_environment, write_auto_switch_rules, write_saved_macros, write_saved_profiles,
    AutoSwitchRule,
    LegacyStreamDockSummary, MacroSequence, Profile, ProfilePage,
};
use drivers::akp05e_protocol::{
    build_touch_strip_packets, parse_input_report, resolve_strip_point,
    HardwareInputEvent, InputControlKind, StripGesture,
};
use drivers::frame_cache::{AntiFreezeFrameCache, FrameCacheStats};
use drivers::hal::{
    builtin_device_catalog, deck_generation, deck_next_report, deck_status, DeckStatus,
    get_cache_stats, key_image_slot, now_ms,
    reset_hardware_knob_leds, scan_usb_devices, send_hardware_brightness, send_hardware_knob_leds,
    send_hardware_led_brightness, send_hardware_led_sleep_mode,
    touch_zone_image_slot, write_animation_frames, write_hid_bursts,
    wake_panel, request_full_push, DeviceDescriptor, DiagnosticEntry,
    HidBurst, PlaybackSlot,
    PlaybackTarget, SyncKeyResult,
};
use media::{decode_b64, MediaPreview, MediaStore};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, State, Manager};

mod commands;
mod discovery;
mod backup;
mod draft;
mod gestures;
mod live;
mod soundboard;
mod macro_run;
mod state;
mod startup_profile;
mod workers;
mod performance;
mod instance;
mod lifecycle;

pub use state::*;
use commands::*;
use live::*;
use macro_run::*;
use workers::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // The elevated helper started by Restart deck: reset the deck's USB port and exit,
    // before taking the single-instance lock the running app holds.
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|arg| arg == "--cycle-deck-port") {
        #[cfg(target_os = "windows")]
        std::process::exit(drivers::hal::cycle_deck_port(args.get(pos + 1).map(String::as_str).unwrap_or("")) as i32);
        #[cfg(not(target_os = "windows"))]
        { let _ = pos; std::process::exit(1); }
    }
    let _instance = match instance::acquire() {
        Ok(Some(guard)) => guard,
        Ok(None) => { instance::focus_existing(); return; }
        Err(err) => { eprintln!("Could not establish exclusive DeckForge instance: {err}"); return; }
    };
    // Pause before state initialization or workers can start a physical transfer.
    if std::env::args().any(|arg| arg == "--start-paused") {
        drivers::hal::pause_deck_control();
    }
    let shared: SharedState = Arc::new(Mutex::new(ManagedAppState::default()));
    let shared_for_setup = Arc::clone(&shared);

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(shared)
        .setup(move |app| {
            let handle = app.handle().clone();
            lifecycle::setup(&handle)?;
            if std::env::args().any(|arg| arg == "--background") {
                if let Some(window) = app.get_webview_window("main") { let _ = window.hide(); }
            }
            spawn_hardware_input_worker(handle.clone(), Arc::clone(&shared_for_setup));
            spawn_animation_worker(Arc::clone(&shared_for_setup));
            let nudge_shared = Arc::clone(&shared_for_setup);
            game_mode::spawn_watcher(
                handle.clone(),
                Arc::new(move || nudge_face_sample(&nudge_shared)),
            );
            plugins::start(handle.clone(), Arc::clone(&shared_for_setup));
            updates::start(handle.clone());
            spawn_desktop_worker(handle, Arc::clone(&shared_for_setup));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide(); // Keep the webview and unsaved editor state alive.
            }
        })
        .invoke_handler(tauri::generate_handler![
            lifecycle::quit_application,
            commands::app_icon,
            commands::open_apps,
            lifecycle::get_launch_at_login,
            lifecycle::set_launch_at_login,
            get_initial_state,
            save_startup_profile,
            draft::get_profile_draft,
            draft::save_profile_draft,
            draft::discard_profile_draft,
            refresh_usb_devices,
            set_device_brightness,
            set_knob_leds,
            set_knob_led_effect,
            reset_knob_leds,
            set_led_brightness,
            set_led_sleep_mode,
            apply_profile_to_device,
            save_profiles,
            save_macros,
            test_key_action,
            execute_macro_sequence,
            stop_macro_execution,
            take_over_from_official_app,
            pause_deck_control,
            restart_deck,
            get_deck_status,
            get_workspace_status,
            resend_layout,
            save_export_file,
            backup::export_profiles_backup,
            backup::restore_profiles_backup,
            backup::export_profile_pack,
            backup::restore_profile_pack,
            media_download::search_commons_gifs,
            media_download::open_commons_file_page,
            import_legacy_sdprofiles,
            get_telemetry,
            get_performance_snapshot,
            hold_auto_switch,
            save_auto_switch_rules,
            save_obs_settings,
            stage_screen_media,
            import_media_url,
            stage_frame_sequence,
            game_mode_scan,
            game_mode_get_config,
            game_mode_save_config,
            game_mode_status,
            game_mode_set,
            game_mode_arm_helper,
            commands::plugin_list,
            commands::plugin_rescan,
            commands::plugin_set_enabled,
            commands::plugin_install_file,
            commands::plugin_github_search,
            commands::plugin_github_install,
            commands::plugin_open_page,
            updates::update_ready,
            commands::audio_apps,
            commands::audio_outputs,
            commands::pick_sound_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running DeckForge Tauri application");
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::drivers::hal::sync_keys_to_hardware;

    // Needs the vendor app's profiles on this machine. Run with `cargo test -- --ignored`.
    #[test]
    #[ignore = "needs the official Stream Dock app's profiles installed on this machine"]
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

    // Sends real commands to a plugged-in AKP05E Pro. Never run it in CI or while the app
    // is running; use `cargo test -- --ignored` on purpose.
    #[test]
    #[ignore = "sends commands to a real deck over USB"]
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

        // The deck is opened by its I/O thread. A write queued before that open is dropped as
        // stale on purpose, so wait for the open and the firmware to settle, as the app does.
        let opened_by = std::time::Instant::now() + Duration::from_secs(10);
        while drivers::hal::deck_generation() == 0 {
            assert!(std::time::Instant::now() < opened_by, "deck did not open within 10 s");
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(1500));

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
