use super::*;

#[tauri::command]
pub async fn get_initial_state(state: State<'_, SharedState>) -> Result<FullBootstrapPayload, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        discovery::refresh(&shared);
        let telemetry = collect_system_telemetry();
        let legacy_summary = scan_legacy_streamdock_environment();
        // Capture editor context and data together after slow OS/file queries.
        let (mut payload, media) = {
            let guard = shared.lock_safe();
            (FullBootstrapPayload {
                applied_context: guard.applied_editor_context(),
                devices: guard.devices.clone(),
                profiles: guard.profiles.clone(),
                auto_switch_rules: guard.auto_switch_rules.clone(),
                macros: guard.macros.clone(),
                diagnostics: guard.diagnostics.clone(),
                frame_cache_stats: get_cache_stats(&guard.frame_cache),
                legacy_summary,
                telemetry,
                saved_locally_at_ms: guard.saved_locally_at_ms,
                applied_to_device_at_ms: guard.applied_to_device_at_ms,
                recent_input_events: guard.recent_input_events.clone(),
                media_library: Vec::new(),
                face: guard.face.clone(),
                obs: guard.obs.clone(),
            }, guard.media.clone())
        };
        payload.media_library = media.previews();
        payload
    }).await.map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn refresh_usb_devices(state: State<'_, SharedState>) -> Result<Vec<DeviceDescriptor>, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (devices, newly_connected) = discovery::refresh(&shared);
        if newly_connected {
            request_full_push("deck found by a rescan");
        }
        devices
    }).await.map_err(|err| err.to_string())
}

#[tauri::command]
pub fn set_device_brightness(
    device_id: String,
    percent: u8,
    state: State<'_, SharedState>,
) -> Result<String, String> {
    let percent = percent.min(100);
    let dev = {
        let mut guard = state.lock_safe();
        let Some(dev) = guard.devices.iter_mut().find(|d| d.id == device_id) else {
            return Err(format!("Device {} not found", device_id));
        };
        dev.brightness = percent;
        let dev = dev.clone();
        dev
    };
    let mut diags = Vec::new();
    let res = send_hardware_brightness(&dev, percent, &mut diags)?;
    { let mut guard = state.lock_safe();
        guard.diagnostics.extend(diags);
    }
    Ok(res)
}

#[tauri::command]
pub async fn apply_profile_to_device(
    device_id: String,
    profile_id: String,
    page_id: String,
    state: State<'_, SharedState>,
) -> Result<SyncKeyResult, String> {

    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _display = DISPLAY_WRITE.lock_safe();
        let prepared = {
            let mut guard = shared.lock_safe();
            let dev = guard.devices.iter().find(|d| d.id == device_id)
                .cloned().ok_or_else(|| format!("Device {device_id} not found"))?;
            let page_index = guard.profiles.iter().find(|p| p.id == profile_id)
                .ok_or_else(|| format!("Profile {profile_id} not found"))?
                .pages.iter().position(|p| p.id == page_id)
                .ok_or_else(|| format!("Page {page_id} not found"))?;
            prepare_layout(&mut guard, dev, &profile_id, page_index, true)?
        };
        transmit_layout(&shared, prepared, write_hid_bursts)
    }).await.map_err(|err| err.to_string())?

}

#[tauri::command]
pub async fn save_profiles(profiles: Vec<Profile>, state: State<'_, SharedState>) -> Result<u64, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || save_profiles_with(&shared, profiles, write_saved_profiles))
        .await.map_err(|err| err.to_string())?
}

// The frontend orders revisions. Keep disk replacement and state publication
// serialized too, without holding shared app state across filesystem work.
static PROFILE_WRITE: Mutex<()> = Mutex::new(());

pub(crate) fn save_profiles_with(
    shared: &SharedState,
    profiles: Vec<Profile>,
    write: impl FnOnce(&[Profile]) -> Result<(), String>,
) -> Result<u64, String> {
    let _write = PROFILE_WRITE.lock_safe();
    write(&profiles)?;
    let ts = now_ms();
    let mut guard = shared.lock_safe();
    guard.profiles = profiles;
    guard.saved_locally_at_ms = ts;
    Ok(ts)
}

#[tauri::command]
pub async fn test_key_action(
    action_id: String,
    action_label: String,
    action_param: String,
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
) -> Result<ActionExecutionResult, String> {
    let action = LiveAction {
        action_id: action_id.clone(),
        label: action_label.clone(),
        param: action_param.clone(),
        note: "Test".to_string(),
        device_id: "action-runner".to_string(),
        key_index: None,
    };
    let shared = state.inner().clone();
    let res = tauri::async_runtime::spawn_blocking(move || dispatch_live_action(&app, &shared, &action))
        .await.map_err(|err| err.to_string())?;
    let mut guard = state.lock_safe();
    guard.diagnostics.push(DiagnosticEntry {
        id: format!("diag-act-{}", res.timestamp_ms),
        timestamp_ms: res.timestamp_ms,
        level: if res.success { "success" } else { "error" }.into(),
        device_id: "action-runner".to_string(),
        stage: "WIN32_ACTION".to_string(),
        message: res.output_message.clone(),
        packet_hex: None,
    });
    Ok(res)
}
#[tauri::command]
pub fn save_macros(macros: Vec<db::MacroSequence>, state: State<'_, SharedState>) -> Result<(), String> {
    let mut guard = state.lock_safe();
    write_saved_macros(&macros)?;
    guard.macros = macros;
    Ok(())
}

#[tauri::command]
pub async fn execute_macro_sequence(
    macro_id: String,
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
) -> Result<MacroRunReport, String> {
    let seq = {
        let guard = state.lock_safe();
        find_macro(&guard, &macro_id).ok_or_else(|| format!("Macro {} not found", macro_id))?
    };
    let permit = reserve_macro(&seq)?;
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || run_macro_sequence(&app, &shared, &seq, permit))
        .await.map_err(|err| err.to_string())
}

#[tauri::command]
pub fn stop_macro_execution() -> String {
    if cancel_running_macro() { "Stop requested. No further macro steps will start.".into() }
    else { "No macro is running.".into() }
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
    use std::io::Write;
    for n in 1..10000 {
        let path = dir.join(if n == 1 { wanted.clone() } else { format!("{stem} ({n}).json") });
        let mut file = match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("Could not create export: {err}")),
        };
        let result = file.write_all(contents.as_bytes()).and_then(|_| file.sync_all());
        drop(file);
        if let Err(err) = result {
            let _ = std::fs::remove_file(&path); // Only the file created by this invocation.
            return Err(format!("Could not finish export: {err}"));
        }
        return Ok(path);
    }
    Err("No unused export filename available".into())
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
    let msg = drivers::hal::resume_deck_control()?;
    let mut guard = state.lock_safe();
    guard.layout_ready = false;
    Ok(msg)
}

/// Restarts the deck like a replug: lets go of it, has Windows reset the USB port it is on
/// (after the user approves the permission prompt), then reconnects and redraws.
#[tauri::command]
pub async fn restart_deck(state: State<'_, SharedState>) -> Result<String, String> {
    let serial = {
        let guard = state.lock_safe();
        guard.devices.iter().find(|d| d.id == "ajazz-akp05e-pro").map(|d| d.serial_number.clone()).unwrap_or_default()
    };
    drivers::hal::deck_usb_instance_id(&serial)?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        drivers::hal::pause_deck_control();
        if !drivers::hal::wait_deck_released(std::time::Duration::from_secs(5)) {
            return Err("DeckForge could not let go of the deck in time. Try again.".to_string());
        }
        drivers::hal::restart_usb_device(&serial)
    })
    .await
    .map_err(|e| e.to_string())?;
    // Take the deck back whatever happened, so a cancelled prompt does not leave it paused.
    let resumed = drivers::hal::resume_deck_control();
    state.lock_safe().layout_ready = false;
    result?;
    resumed?;
    Ok("Deck restarted. DeckForge is reconnecting and redrawing it.".into())
}

#[tauri::command]
pub fn pause_deck_control() -> String {
    drivers::hal::pause_deck_control()
}

#[tauri::command]
pub async fn import_legacy_sdprofiles(state: State<'_, SharedState>) -> Result<Vec<Profile>, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || import_legacy_with(&shared, parse_all_legacy_sdprofiles))
        .await.map_err(|err| err.to_string())
}

pub(crate) fn import_legacy_with(
    shared: &SharedState,
    parse: impl FnOnce() -> Vec<Profile>,
) -> Vec<Profile> {
    let imported = parse();
    let mut guard = shared.lock_safe();
    guard.diagnostics.push(DiagnosticEntry {
        id: format!("diag-import-{}", now_ms()),
        timestamp_ms: now_ms(),
        level: "info".to_string(),
        device_id: "profile-store".to_string(),
        stage: "SDPROFILE_IMPORT".to_string(),
        message: format!("Parsed {} legacy .sdProfile configurations for editor import; save to keep them", imported.len()),
        packet_hex: None,
    });
    let excess = guard.diagnostics.len().saturating_sub(80);
    guard.diagnostics.drain(..excess);
    // Import is an editor draft. Never replace or extend the saved backend
    // snapshot; the frontend merges these candidates into its latest edits.
    imported
}

#[tauri::command]
pub async fn get_telemetry() -> Result<SystemTelemetrySnapshot, String> {
    // Enumerating processes can be slow; never run it on the window thread.
    tauri::async_runtime::spawn_blocking(|| {
        let _timing = performance::timer(performance::Stage::Telemetry);
        collect_system_telemetry()
    })
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn hold_auto_switch(state: State<'_, SharedState>) -> Result<(), String> {
    let mut guard = state.lock_safe();
    guard.auto_switch_hold_until_ms = now_ms().saturating_add(6000);
    Ok(())
}

#[tauri::command]
pub fn save_auto_switch_rules(rules: Vec<AutoSwitchRule>, state: State<'_, SharedState>) -> Result<(), String> {
    let mut guard = state.lock_safe();
    write_auto_switch_rules(&rules)?;
    guard.auto_switch_rules = rules;
    Ok(())
}

#[tauri::command]
pub fn save_obs_settings(settings: ObsSettings, state: State<'_, SharedState>) -> Result<(), String> {
    let mut guard = state.lock_safe();
    settings.save()?;
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
    let mut guard = state.lock_safe();
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
    let mut guard = state.lock_safe();
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

/// Compare the editor draft with the last successful transfer, never the saved draft.
#[tauri::command]
pub fn get_workspace_status(device_id: String, profile_id: String, page: ProfilePage, state: State<'_, SharedState>) -> WorkspaceStatus {
    let applied = state.lock_safe().applied_page_status(&device_id, &profile_id, &page);
    WorkspaceStatus { deck: deck_status(), applied }
}

/// Icon of a running app as a PNG data URL, for auto-switch rules. None when it isn't running.
#[tauri::command]
pub async fn app_icon(process_name: String) -> Option<String> {
    // Searching install folders can take a moment, so keep it off the UI thread.
    tauri::async_runtime::spawn_blocking(move || actions::app_icon_data_url(&process_name))
        .await
        .ok()
        .flatten()
}

#[tauri::command]
pub async fn open_apps() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(crate::game_mode::open_app_exes).await.unwrap_or_default()
}

#[tauri::command]
pub fn plugin_list() -> Vec<crate::plugins::PluginView> {
    crate::plugins::list()
}

#[tauri::command]
pub async fn plugin_rescan() -> Vec<crate::plugins::PluginView> {
    crate::plugins::rescan()
}

/// Async so starting an HTML plugin's hidden window never runs on the main thread.
#[tauri::command]
pub async fn plugin_set_enabled(uuid: String, enabled: bool) -> Result<Vec<crate::plugins::PluginView>, String> {
    crate::plugins::set_enabled(&uuid, enabled)
}

/// Installs a `.streamDeckPlugin` file the user picked, sent as base64.
#[tauri::command]
pub async fn plugin_install_file(data: String) -> Result<crate::plugins::Installed, String> {
    use base64::Engine as _;
    if data.len() > crate::plugins::MAX_UPLOAD_B64 {
        return Err("The plugin file is too large.".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(data.trim()).map_err(|_| "Could not read that file.".to_string())?;
    tauri::async_runtime::spawn_blocking(move || crate::plugins::install_archive(&bytes)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn plugin_github_search(query: String) -> Result<Vec<crate::plugins::GithubPlugin>, String> {
    crate::plugins::search_github(&query).await
}

#[tauri::command]
pub async fn plugin_github_install(repo: String) -> Result<crate::plugins::Installed, String> {
    crate::plugins::install_from_github(&repo).await
}

/// Opens a plugin's GitHub page in the browser.
#[tauri::command]
pub fn plugin_open_page(url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/") {
        return Err("Only GitHub pages can be opened here.".into());
    }
    crate::actions::launch_app_or_url(&url).map(|_| ())
}

/// Sound outputs, for the "Switch sound output" key.
#[tauri::command]
pub async fn audio_outputs() -> Result<Vec<crate::actions::AudioOutput>, String> {
    tauri::async_runtime::spawn_blocking(crate::actions::list_outputs).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) fn get_performance_snapshot(reset: bool) -> performance::Snapshot {
    performance::snapshot(reset)
}

#[tauri::command]
pub async fn audio_apps() -> Result<Vec<actions::app_audio::AudioApp>, String> {
    tauri::async_runtime::spawn_blocking(actions::app_audio::list).await.map_err(|e| e.to_string())?
}
