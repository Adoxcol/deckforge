use super::*;

#[tauri::command]
pub fn get_initial_state(state: State<'_, SharedState>) -> Result<FullBootstrapPayload, String> {
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
pub fn refresh_usb_devices(state: State<'_, SharedState>) -> Result<Vec<DeviceDescriptor>, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let mut devices = guard.devices.clone();
    let mut diags = guard.diagnostics.clone();
    let was_connected = devices.iter().any(|d| d.connected && d.protocol_family.starts_with("CRT"));
    scan_usb_devices(&mut devices, &mut diags);
    let now_connected = devices.iter().any(|d| d.connected && d.protocol_family.starts_with("CRT"));
    guard.devices = devices.clone();
    guard.diagnostics = diags;
    drop(guard);
    if now_connected && !was_connected {
        // A deck that only now shows up has not had its pictures sent.
        request_full_push("deck found by a rescan");
    }
    Ok(devices)
}

#[tauri::command]
pub fn set_device_brightness(
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
pub fn apply_profile_to_device(
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
pub fn clear_device_key(
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
pub fn save_profiles(profiles: Vec<Profile>, state: State<'_, SharedState>) -> Result<u64, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    let ts = now_ms();
    write_saved_profiles(&profiles)?;
    guard.profiles = profiles;
    guard.saved_locally_at_ms = ts;
    Ok(ts)
}

#[tauri::command]
pub fn test_key_action(
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
#[tauri::command]
pub fn save_macros(macros: Vec<db::MacroSequence>, state: State<'_, SharedState>) -> Result<(), String> {
    write_saved_macros(&macros)?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.macros = macros;
    Ok(())
}

#[tauri::command]
pub fn execute_macro_sequence(
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

/// Connection state and how the last send went, polled by the status bar.
#[tauri::command]
pub fn get_deck_status() -> DeckStatus {
    deck_status()
}

/// Sends every picture again with the full connect sequence. Use it when the deck is blank
/// or shows the wrong pictures. Apply to Device does not do the connect sequence.
#[tauri::command]
pub fn resend_layout() -> String {
    request_full_push("requested from the app");
    "Sending the whole layout to the deck again.".to_string()
}

/// Where exported profiles and modes are written: `Documents\DeckForge Exports`.
fn exports_dir() -> std::path::PathBuf {
    use std::path::PathBuf;
    let profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
    let documents = profile
        .as_ref()
        .map(|p| p.join("Documents"))
        .filter(|p| p.is_dir())
        .or_else(|| {
            std::env::var_os("OneDrive")
                .map(|p| PathBuf::from(p).join("Documents"))
                .filter(|p| p.is_dir())
        })
        .or(profile)
        .unwrap_or_else(std::env::temp_dir);
    documents.join("DeckForge Exports")
}

/// Keeps a user-chosen name safe to use as a Windows file name.
fn safe_export_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')') { c } else { '-' })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_string();
    let base = if trimmed.is_empty() { "deckforge-export".to_string() } else { trimmed };
    let base = base.strip_suffix(".json").unwrap_or(&base).to_string();
    let base: String = base.chars().take(80).collect();
    format!("{base}.json")
}

/// Writes `contents` into `dir` under a safe version of `file_name`. Never overwrites: a name
/// that exists gets a number.
fn write_export(dir: &std::path::Path, file_name: &str, contents: &str) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let wanted = safe_export_name(file_name);
    let stem = wanted.trim_end_matches(".json").to_string();
    let mut path = dir.join(&wanted);
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}).json"));
        n += 1;
    }
    std::fs::write(&path, contents).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(path)
}

/// Writes a share file (profile or Game Mode) into the exports folder and shows it in Explorer.
#[tauri::command]
pub fn save_export_file(file_name: String, contents: String) -> Result<String, String> {
    let path = write_export(&exports_dir(), &file_name, &contents)?;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn();
    }
    Ok(path.display().to_string())
}

#[cfg(test)]
mod export_tests {
    use super::{safe_export_name, write_export};

    #[test]
    fn exports_never_overwrite_an_existing_file() {
        let dir = std::env::temp_dir().join(format!("deckforge-export-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first = write_export(&dir, "My Profile.json", "one").unwrap();
        let second = write_export(&dir, "My Profile", "two").unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "one");
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "two");
        assert!(second.file_name().unwrap().to_string_lossy().ends_with("(2).json"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_names_are_safe_and_end_in_json() {
        assert_eq!(safe_export_name("Streaming / Main?"), "Streaming - Main-.json");
        assert_eq!(safe_export_name("profile.json"), "profile.json");
        assert_eq!(safe_export_name("  ..  "), "deckforge-export.json");
        let evil = safe_export_name("..\\..\\evil");
        assert!(!evil.contains('\\') && !evil.contains('/') && evil.ends_with(".json"));
    }
}

#[tauri::command]
pub fn take_over_from_official_app(state: State<'_, SharedState>) -> Result<String, String> {
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
pub fn import_legacy_sdprofiles(state: State<'_, SharedState>) -> Result<Vec<Profile>, String> {
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
pub fn get_telemetry() -> SystemTelemetrySnapshot {
    collect_system_telemetry()
}

#[tauri::command]
pub fn set_active_context(
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
pub fn hold_auto_switch(state: State<'_, SharedState>) -> Result<(), String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.auto_switch_hold_until_ms = now_ms().saturating_add(6000);
    Ok(())
}

#[tauri::command]
pub fn save_auto_switch_rules(rules: Vec<AutoSwitchRule>, state: State<'_, SharedState>) -> Result<(), String> {
    write_auto_switch_rules(&rules)?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.auto_switch_rules = rules;
    Ok(())
}

#[tauri::command]
pub fn save_obs_settings(settings: ObsSettings, state: State<'_, SharedState>) -> Result<(), String> {
    settings.save()?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.obs = settings;
    Ok(())
}

#[tauri::command]
pub fn stage_screen_media(
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
pub fn stage_frame_sequence(
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
pub async fn game_mode_scan() -> Result<game_mode::ScanReport, String> {
    tauri::async_runtime::spawn_blocking(game_mode::scan)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn game_mode_get_config() -> game_mode::GameModeConfig {
    game_mode::config()
}

#[tauri::command]
pub fn game_mode_save_config(config: game_mode::GameModeConfig) -> Result<game_mode::GameModeConfig, String> {
    game_mode::save_config(config)
}

#[tauri::command]
pub fn game_mode_status() -> game_mode::GameModeStatus {
    game_mode::status()
}

/// Turns Game Mode on or off. `enabled: None` flips it. Work happens on a thread and the
/// result arrives through the `game-mode` event.
#[tauri::command]
pub fn game_mode_set(
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
pub fn game_mode_arm_helper(app: tauri::AppHandle) -> Result<String, String> {
    if game_mode::is_busy() {
        return Err("Game Mode is still working.".to_string());
    }
    std::thread::spawn(move || {
        let message = game_mode::arm(&app);
        let _ = app.emit("game-mode-message", &message);
    });
    Ok("Starting the admin helper".to_string())
}
