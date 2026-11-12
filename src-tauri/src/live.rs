use super::*;

pub(crate) struct LiveAction {
    pub(crate) action_id: String,
    pub(crate) label: String,
    pub(crate) param: String,
    pub(crate) note: String,
    pub(crate) device_id: String,
}

pub(crate) fn kind_code(kind: &InputControlKind) -> u8 {
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

pub(crate) fn action_for_event(guard: &ManagedAppState, evt: &HardwareInputEvent) -> Option<LiveAction> {
    if !guard.layout_ready {
        return None;
    }
    let page = &guard.applied_layout.as_ref()?.page;
    match evt.kind {
        InputControlKind::LcdKey => {
            let key = page.keys.iter().find(|key| key.index == evt.control_index)?;
            Some(LiveAction {
                action_id: key.action_id.clone(),
                label: key.action_label.clone(),
                param: key.action_param.clone(),
                note: format!("Key #{} ({})", evt.control_index + 1, key.title),
                device_id: evt.device_id.clone(),
            })
        }
        InputControlKind::RotaryKnobTurnCw => {
            let encoder = page.encoders.iter().find(|encoder| encoder.index == evt.control_index);
            Some(binding_action(
                encoder.map(|e| e.cw_action.as_str()).unwrap_or("audio.volume_up"),
                encoder.map(|e| e.cw_param.as_str()).unwrap_or(""),
                "Knob clockwise",
                format!("Knob #{} clockwise", evt.control_index + 1),
                evt.device_id.clone(),
            ))
        }
        InputControlKind::RotaryKnobTurnCcw => {
            let encoder = page.encoders.iter().find(|encoder| encoder.index == evt.control_index);
            Some(binding_action(
                encoder.map(|e| e.ccw_action.as_str()).unwrap_or("audio.volume_down"),
                encoder.map(|e| e.ccw_param.as_str()).unwrap_or(""),
                "Knob counter-clockwise",
                format!("Knob #{} counter-clockwise", evt.control_index + 1),
                evt.device_id.clone(),
            ))
        }
        InputControlKind::RotaryKnobPush => {
            let encoder = page.encoders.iter().find(|encoder| encoder.index == evt.control_index);
            Some(binding_action(
                encoder.map(|e| e.push_action.as_str()).unwrap_or("audio.volume_mute"),
                encoder.map(|e| e.push_param.as_str()).unwrap_or(""),
                "Knob press",
                format!("Knob #{} press", evt.control_index + 1),
                evt.device_id.clone(),
            ))
        }
        InputControlKind::TouchStripTap => {
            let zone = page.touch_zones.iter().find(|zone| zone.index == evt.control_index)?;
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

pub(crate) fn known_action(binding: &str) -> bool {
    // `hotkey.<name>` is the preset family from the Shortcuts catalog. The key combo itself
    // travels in the action parameter, so every preset shares one executor arm.
    binding.starts_with("hotkey.") || matches!(
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
            | "obs.toggle_virtualcam"
            | "obs.toggle_replay"
            | "obs.pause_record"
            | "obs.studio_transition"
            | "obs.set_scene"
            | "obs.toggle_mute"
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

pub(crate) fn binding_action(binding: &str, param: &str, label: &str, note: String, device_id: String) -> LiveAction {
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

pub(crate) fn face_changes(action_id: &str) -> bool {
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

pub(crate) fn lcd_title(raw: &str) -> String {
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

pub(crate) fn key_level(action_id: &str, face: &DeckFaceState) -> Option<u8> {
    if matches!(action_id, "audio.volume_up" | "audio.volume_down" | "audio.volume_mute") {
        face.volume_percent
    } else {
        None
    }
}

pub(crate) fn styled_key(key: &KeyConfig, face: &DeckFaceState) -> (String, String) {
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

pub(crate) fn styled_zone_value(zone: &TouchZoneConfig, face: &DeckFaceState) -> String {
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

pub(crate) fn plan_screens(dev: &DeviceDescriptor, page: &ProfilePage, face: &DeckFaceState) -> (Vec<ScreenJob>, Vec<ZoneJob>, Option<StripJob>) {
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

pub(crate) fn apply_brightness_action(
    shared: &SharedState,
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
    let _ = write_panel_brightness(&dev, percent);
    Some(format!("Backlight set to {percent}%"))
}

/// Streams the saved layout after the deck has accepted the handshake.
/// These panels keep no picture of their own; a later handshake leaves them black until this runs.
/// The device list is read once when the app starts. If the deck was rebooting then (its
/// firmware restarts when the app dies without a heartbeat), the list says "not connected"
/// even though the IO thread opens the deck a few seconds later, and the layout would never
/// be sent. So look again before the first push.
fn ensure_deck_in_state(shared: &SharedState) -> bool {
    for attempt in 0..6u32 {
        {
            let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
            let known = |devices: &[DeviceDescriptor]| devices.iter().any(|d| d.connected && d.protocol_family.starts_with("CRT"));
            if known(&guard.devices) {
                return true;
            }
            let mut devices = guard.devices.clone();
            let mut diagnostics = Vec::new();
            scan_usb_devices(&mut devices, &mut diagnostics);
            let found = known(&devices);
            guard.devices = devices;
            guard.diagnostics.extend(diagnostics);
            if found {
                drop(guard);
                deck_note("deck was missing from the device list at start; found it on a later scan");
                return true;
            }
        }
        if attempt < 5 {
            std::thread::sleep(Duration::from_millis(700));
        }
    }
    false
}

pub(crate) fn push_connected_layout(shared: &SharedState) {
    if !ensure_deck_in_state(shared) {
        deck_note("no deck in the device list, so the layout was not sent");
        return;
    }
    // First contact may find the panel blank, so this one path handshakes and
    // then restores brightness. Later page changes must not handshake.
    //
    // A failed send is tried again. When the deck dropped off USB in the middle of it, the
    // IO thread opens it again and the input worker sends the layout from scratch, so
    // this loop gives up as soon as the deck has been reopened.
    let generation = deck_generation();
    for attempt in 1..=3u32 {
        match restore_connected_layout(shared) {
            Ok(()) => return,
            Err(err) => {
                deck_note(&format!("layout push failed (attempt {attempt}/3): {err}"));
                std::thread::sleep(Duration::from_millis(2500));
                if deck_generation() != generation {
                    return;
                }
            }
        }
    }
}

pub(crate) fn restore_connected_layout(shared: &SharedState) -> Result<(), String> {
    let _display = DISPLAY_WRITE.lock_safe();
    let prepared = prepare_reconnect(&mut shared.lock_safe())?;
    transmit_layout(shared, prepared, write_hid_bursts).map(|_| ())
}

fn prepare_reconnect(guard: &mut ManagedAppState) -> Result<PreparedLayout, String> {
    let dev = guard.devices.iter()
        .find(|d| d.connected && device_uses_akp05e_face(d))
        .cloned().ok_or("No connected deck")?;
    if let Some(applied) = guard.applied_layout.clone() {
        // Resend the exact applied page, not saved edits which have not been applied.
        return prepare_snapshot(guard, dev, applied, true);
    }
    let profile_id = guard.active_profile_id.clone();
    let page_index = guard.active_page_index;
    prepare_layout(guard, dev, &profile_id, page_index, true)
}

pub(crate) fn push_active_layout_locked(shared: &SharedState, handshake: bool) -> Result<(), String> {
    let prepared = {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        let dev = guard.devices.iter()
            .find(|d| d.connected && d.protocol_family.starts_with("CRT"))
            .cloned().ok_or("No connected deck")?;
        let profile_id = guard.active_profile_id.clone();
        let page_index = guard.active_page_index;
        prepare_layout(&mut guard, dev, &profile_id, page_index, handshake)?
    };
    transmit_layout(shared, prepared, |dev, bursts| {
        if handshake { write_hid_bursts(dev, bursts) } else { write_animation_frames(dev, bursts) }
    }).map(|_| ())
}

pub(crate) struct PreparedLayout {
    dev: DeviceDescriptor,
    applied: AppliedLayout,
    outcome: drivers::hal::PublishOutcome,
}

/// Callers hold DISPLAY_WRITE until transmit_layout finishes.
pub(crate) fn prepare_layout(
    guard: &mut ManagedAppState,
    dev: DeviceDescriptor,
    profile_id: &str,
    page_index: usize,
    invalidate: bool,
) -> Result<PreparedLayout, String> {
    if !dev.connected || dev.is_simulator || !device_uses_akp05e_face(&dev) {
        return Err("A connected AKP05E Pro is required to apply a layout".into());
    }
    let prof = guard
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile {profile_id} not found"))?;
    let page = prof
        .pages
        .get(page_index)
        .ok_or_else(|| format!("Page index {page_index} not found"))?
        .clone();
    prepare_snapshot(guard, dev, AppliedLayout { profile_id: profile_id.to_string(), page_index, page }, invalidate)
}

fn prepare_snapshot(
    guard: &mut ManagedAppState,
    dev: DeviceDescriptor,
    applied: AppliedLayout,
    invalidate: bool,
) -> Result<PreparedLayout, String> {
    if !dev.connected || dev.is_simulator || !device_uses_akp05e_face(&dev) {
        return Err("A connected AKP05E Pro is required to apply a layout".into());
    }
    if invalidate || !guard.layout_ready {
        guard.frame_cache.invalidate_device(&dev.id);
    }
    // During a partial transfer the panel may contain a mix of old and new images.
    // Do not resolve physical presses until the entire layout succeeds.
    guard.layout_ready = false;
    guard.playback_slots.clear();
    let (jobs, zones, strip) = plan_screens(&dev, &applied.page, &guard.face);
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
    Ok(PreparedLayout {
        dev,
        applied,
        outcome,
    })
}

pub(crate) fn transmit_layout(
    shared: &SharedState,
    prepared: PreparedLayout,
    write: impl FnOnce(&DeviceDescriptor, &[HidBurst]) -> Result<(), String>,
) -> Result<SyncKeyResult, String> {
    let result = write(&prepared.dev, &prepared.outcome.bursts);
    let mut guard = shared.lock_safe();
    if let Err(err) = result {
        guard.frame_cache.invalidate_device(&prepared.dev.id);
        guard.layout_ready = false;
        guard.applied_to_device_at_ms = None;
        // Keep the last successful snapshot for recovery, but do not dispatch it
        // against a panel which may now show only part of the attempted layout.
        if let Some(applied) = guard.applied_layout.clone() {
            guard.active_profile_id = applied.profile_id;
            guard.active_page_index = applied.page_index;
        }
        return Err(err);
    }
    guard.active_profile_id = prepared.applied.profile_id.clone();
    guard.active_page_index = prepared.applied.page_index;
    guard.applied_layout = Some(prepared.applied);
    guard.layout_ready = true;
    guard.playback_slots = prepared.outcome.playback;
    guard.playback_device_id = prepared.dev.id;
    guard.applied_to_device_at_ms = Some(now_ms());
    let mut result = prepared.outcome.result;
    if let Err(err) = guard.persist_applied_layout() {
        let warning = format!("Layout sent, but could not save it for restart: {err}");
        let device_id = guard.playback_device_id.clone();
        guard.diagnostics.push(DiagnosticEntry {
            id: format!("diag-layout-save-{}", now_ms()), timestamp_ms: now_ms(),
            level: "warning".into(), device_id,
            stage: "SAVE_APPLIED_LAYOUT".into(), message: warning.clone(), packet_hex: None,
        });
        result.status_message = warning;
    }
    Ok(result)
}

pub(crate) fn dispatch_live_action(
    app: &tauri::AppHandle,
    shared: &SharedState,
    action: &LiveAction,
) -> String {
    if action.action_id == "macro.run" {
        return start_macro(app, shared, &action.param, &action.label);
    }
    if action.action_id == "sys.game_mode" {
        return start_game_mode_toggle(app, shared, &action.param);
    }
    if let Some(message) = apply_brightness_action(shared, &action.action_id) {
        return message;
    }
    if let Some(message) = apply_navigation(app, shared, &action.action_id, &action.param) {
        return message;
    }
    let obs = shared
        .lock()
        .ok()
        .map(|guard| guard.obs.clone())
        .unwrap_or_default();
    let result = execute_key_action_live(&action.action_id, &action.label, &action.param, true, &obs);
    if face_changes(&action.action_id) {
        { let mut guard = shared.lock_safe();
            guard.sample_after_ms = now_ms().saturating_add(160);
        }
    }
    result.output_message
}

/// Nudges the key faces so the Game Mode key flips to GAME ON or back quickly.
pub(crate) fn nudge_face_sample(shared: &SharedState) {
    { let mut guard = shared.lock_safe();
        guard.sample_after_ms = now_ms().saturating_add(120);
    }
}

/// Game Mode closes apps one at a time, so it never runs on the input thread. The key's
/// parameter picks the mode by id; an empty one uses the mode chosen in the Game Mode tab.
pub(crate) fn start_game_mode_toggle(app: &tauri::AppHandle, shared: &SharedState, mode: &str) -> String {
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

pub(crate) fn apply_navigation(
    app: &tauri::AppHandle,
    shared: &SharedState,
    action_id: &str,
    param: &str,
) -> Option<String> {
    if !matches!(
        action_id,
        "profile.next_page" | "profile.prev_page" | "profile.switch_page" | "profile.switch"
    ) {
        return None;
    }
    let _display = DISPLAY_WRITE.lock_safe();
    let event = {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        let message = switch_active_page(&mut guard, action_id, param)?;
        DeckContextEvent {
            profile_id: guard.active_profile_id.clone(),
            page_index: guard.active_page_index,
            message,
        }
    };
    if let Err(err) = push_active_layout_locked(shared, false) {
        return Some(format!("Page switch failed: {err}"));
    }
    let _ = app.emit("deck-context", &event);
    Some(event.message)
}

pub(crate) fn switch_active_page(guard: &mut ManagedAppState, action_id: &str, param: &str) -> Option<String> {
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

pub(crate) fn resolve_page_index(guard: &ManagedAppState, param: &str) -> Option<usize> {
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

pub(crate) fn sample_face(guard: &mut ManagedAppState) -> bool {
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

pub(crate) fn push_face_deltas(shared: &SharedState, app: &tauri::AppHandle) {
    let _display = DISPLAY_WRITE.lock_safe();
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
    let _ = write_animation_frames(&dev, &bursts);
}

pub(crate) fn face_delta_bursts(guard: &mut ManagedAppState, face: &DeckFaceState) -> Option<(DeviceDescriptor, Vec<HidBurst>)> {
    if !guard.layout_ready { return None; }
    let dev = guard
        .devices
        .iter()
        .find(|d| d.connected && d.protocol_family.starts_with("CRT"))
        .cloned()?;
    let page = guard.applied_layout.as_ref()?.page.clone();
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

pub(crate) fn rule_matches(rule: &AutoSwitchRule, exe: &str, title: &str) -> bool {
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


#[cfg(test)]
mod live_tests {
    use super::*;

    // No HID discovery, user files, threads, or native actions in these tests.
    fn test_state() -> SharedState {
        let mut devices = builtin_device_catalog();
        devices[0].connected = true;
        Arc::new(Mutex::new(ManagedAppState {
            devices,
            profiles: db::default_profiles(),
            active_profile_id: "profile-streaming".into(),
            active_page_index: 0,
            applied_layout: None,
            applied_layout_path: None,
            layout_ready: false,
            auto_switch_rules: Vec::new(),
            macros: Vec::new(),
            diagnostics: Vec::new(),
            recent_input_events: Vec::new(),
            frame_cache: AntiFreezeFrameCache::new(),
            saved_locally_at_ms: 0,
            applied_to_device_at_ms: None,
            media: MediaStore::default(),
            playback_slots: Vec::new(),
            playback_device_id: String::new(),
            face: DeckFaceState::default(),
            obs: ObsSettings::default(),
            sample_after_ms: 0,
            auto_switch_hold_until_ms: 0,
        }))
    }

    fn prepare_first(shared: &SharedState) -> PreparedLayout {
        let mut guard = shared.lock_safe();
        let dev = guard.devices[0].clone();
        let id = guard.profiles[0].id.clone();
        prepare_layout(&mut guard, dev, &id, 0, true).unwrap()
    }

    fn press(kind: InputControlKind, index: u8) -> HardwareInputEvent {
        HardwareInputEvent {
            device_id: "ajazz-akp05e-pro".into(), kind, control_index: index,
            state_code: 1, pressed: true, raw_hex: String::new(), timestamp_ms: 1, x: 0,
        }
    }

    struct TestLayoutStore(std::path::PathBuf);

    impl TestLayoutStore {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let suffix = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("deckforge-layout-{}-{}-{suffix}", std::process::id(), now_ms()));
            std::fs::create_dir(&dir).unwrap();
            Self(dir.join("applied-layout.json"))
        }
    }

    impl Drop for TestLayoutStore {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_file(self.0.with_extension("json.tmp"));
            let _ = std::fs::remove_dir(self.0.parent().unwrap());
        }
    }

    fn apply_non_default(shared: &SharedState) -> AppliedLayout {
        let prepared = {
            let mut guard = shared.lock_safe();
            let mut profile = guard.profiles[0].clone();
            profile.id = "profile-recovery-test".into();
            let mut page = profile.pages[0].clone();
            page.id = "page-recovery-test".into();
            page.keys[0].title = "Recovery test".into();
            page.keys[0].action_id = "custom.hotkey".into();
            page.keys[0].action_param = "Ctrl+Shift+R".into();
            profile.pages.push(page);
            let index = profile.pages.len() - 1;
            let id = profile.id.clone();
            guard.profiles.push(profile);
            let dev = guard.devices[0].clone();
            prepare_layout(&mut guard, dev, &id, index, true).unwrap()
        };
        transmit_layout(shared, prepared, |_, _| Ok(())).unwrap();
        shared.lock_safe().applied_layout.clone().unwrap()
    }

    #[test]
    fn reconnect_and_resend_keep_non_default_page_despite_new_saved_edits() {
        let shared = test_state();
        let applied = apply_non_default(&shared);
        // Neither edited nor deleted saved pages may alter a recovery transmission.
        shared.lock_safe().profiles.clear();
        for _ in 0..2 {
            let prepared = prepare_reconnect(&mut shared.lock_safe()).unwrap();
            assert_eq!(serde_json::to_string(&prepared.applied).unwrap(), serde_json::to_string(&applied).unwrap());
            transmit_layout(&shared, prepared, |_, bursts| {
                assert!(!bursts.is_empty(), "recovery must resend even unchanged frames");
                Ok(())
            }).unwrap();
            let guard = shared.lock_safe();
            assert_eq!(guard.active_profile_id, applied.profile_id);
            assert_eq!(guard.active_page_index, applied.page_index);
        }
    }

    #[test]
    fn restart_restores_persisted_snapshot_and_failed_apply_does_not_replace_it() {
        let store = TestLayoutStore::new();
        let shared = test_state();
        shared.lock_safe().applied_layout_path = Some(store.0.clone());
        let applied = apply_non_default(&shared);
        let loaded = load_applied_layout(&store.0).unwrap().unwrap();
        assert_eq!(serde_json::to_string(&loaded).unwrap(), serde_json::to_string(&applied).unwrap());
        let failed = prepare_first(&shared);
        assert!(transmit_layout(&shared, failed, |_, _| Err("USB interrupted".into())).is_err());
        let restarted = test_state();
        restarted.lock_safe().applied_layout = load_applied_layout(&store.0).unwrap();
        let prepared = prepare_reconnect(&mut restarted.lock_safe()).unwrap();
        assert_eq!(prepared.applied.profile_id, applied.profile_id);
        assert_eq!(prepared.applied.page.id, applied.page.id);
        transmit_layout(&restarted, prepared, |_, _| Ok(())).unwrap();
        let event = press(InputControlKind::LcdKey, 0);
        assert_eq!(action_for_event(&restarted.lock_safe(), &event).unwrap().param, "Ctrl+Shift+R");
        // A second successful apply replaces the file, including on Windows.
        let prepared = prepare_first(&shared);
        transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        assert_ne!(load_applied_layout(&store.0).unwrap().unwrap().profile_id, applied.profile_id);
    }

    #[test]
    fn missing_corrupt_and_future_layout_stores_are_handled_explicitly() {
        let store = TestLayoutStore::new();
        assert!(load_applied_layout(&store.0).unwrap().is_none());
        std::fs::write(&store.0, "broken JSON").unwrap();
        assert!(load_applied_layout(&store.0).is_err());
        let shared = test_state();
        let applied = apply_non_default(&shared);
        std::fs::write(&store.0, serde_json::json!({"version": 99, "layout": applied}).to_string()).unwrap();
        assert!(load_applied_layout(&store.0).is_err());
        // With no prior snapshot, first connection still applies the initial profile.
        assert!(prepare_reconnect(&mut test_state().lock_safe()).is_ok());
    }

    #[test]
    fn restart_save_failure_is_reported_without_undoing_successful_hardware_apply() {
        let store = TestLayoutStore::new();
        std::fs::write(&store.0, "a file cannot be a parent directory").unwrap();
        let shared = test_state();
        shared.lock_safe().applied_layout_path = Some(store.0.join("unwritable.json"));
        let prepared = prepare_first(&shared);
        let result = transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        assert!(result.status_message.contains("could not save it for restart"));
        assert!(shared.lock_safe().layout_ready);
        assert_eq!(shared.lock_safe().diagnostics.last().unwrap().level, "warning");
    }

    #[test]
    fn startup_editor_context_finds_applied_page_by_id_after_reordering() {
        let shared = test_state();
        let applied = apply_non_default(&shared);
        let mut guard = shared.lock_safe();
        let profile = guard.profiles.iter_mut().find(|p| p.id == applied.profile_id).unwrap();
        profile.pages.swap(0, applied.page_index);
        let context = guard.applied_editor_context().unwrap();
        assert_eq!(context.profile_id, applied.profile_id);
        assert_eq!(context.page_index, 0);
        guard.profiles.clear();
        assert!(guard.applied_editor_context().is_none());
    }

    #[test]
    fn saved_edits_do_not_change_applied_key_encoder_or_touch_bindings() {
        let shared = test_state();
        let prepared = prepare_first(&shared);
        transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        let mut guard = shared.lock_safe();
        let events = [press(InputControlKind::LcdKey, 0),
            press(InputControlKind::RotaryKnobTurnCw, 0),
            press(InputControlKind::TouchStripTap, 0)];
        let before: Vec<_> = events.iter().map(|e| {
            let action = action_for_event(&guard, e).unwrap();
            (action.action_id, action.param)
        }).collect();
        let page = &mut guard.profiles[0].pages[0];
        page.keys[0].action_id = "custom.hotkey".into();
        page.keys[0].action_param = "Alt+F4".into();
        page.encoders[0].cw_action = "custom.hotkey".into();
        page.encoders[0].cw_param = "Alt+F4".into();
        page.touch_zones[0].tap_action_id = "custom.hotkey".into();
        page.touch_zones[0].tap_action_param = "Alt+F4".into();
        // Even replacing the saved collection cannot replace the applied snapshot.
        guard.profiles.clear();
        for (event, expected) in events.iter().zip(before) {
            let action = action_for_event(&guard, event).unwrap();
            assert_eq!((action.action_id, action.param), expected);
        }
    }

    #[test]
    fn apply_commits_the_transmitted_snapshot_even_if_saved_data_changes_during_write() {
        let shared = test_state();
        let prepared = prepare_first(&shared);
        let expected = prepared.applied.page.keys[0].action_id.clone();
        let event = press(InputControlKind::LcdKey, 0);
        assert!(action_for_event(&shared.lock_safe(), &event).is_none());
        transmit_layout(&shared, prepared, |_, bursts| {
            assert!(!bursts.is_empty());
            let mut guard = shared.lock_safe();
            assert!(!guard.layout_ready);
            assert!(guard.applied_to_device_at_ms.is_none());
            guard.profiles[0].pages[0].keys[0].action_id = "custom.hotkey".into();
            Ok(())
        }).unwrap();
        let guard = shared.lock_safe();
        assert_eq!(action_for_event(&guard, &event).unwrap().action_id, expected);
        assert!(guard.applied_to_device_at_ms.is_some());
    }

    #[test]
    fn partial_transfer_disables_actions_until_a_successful_retry() {
        let shared = test_state();
        let prepared = prepare_first(&shared);
        transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        shared.lock_safe().profiles[0].pages[0].keys[0].action_id = "custom.hotkey".into();
        let prepared = prepare_first(&shared);
        assert!(transmit_layout(&shared, prepared, |_, _| Err("USB disconnected".into())).is_err());
        let event = press(InputControlKind::LcdKey, 0);
        {
            let guard = shared.lock_safe();
            assert!(action_for_event(&guard, &event).is_none());
            assert!(guard.applied_to_device_at_ms.is_none());
            assert_ne!(guard.applied_layout.as_ref().unwrap().page.keys[0].action_id, "custom.hotkey");
        }
        let prepared = {
            let mut guard = shared.lock_safe();
            let dev = guard.devices[0].clone();
            let id = guard.profiles[0].id.clone();
            prepare_layout(&mut guard, dev, &id, 0, false).unwrap()
        };
        transmit_layout(&shared, prepared, |_, bursts| {
            assert!(!bursts.is_empty(), "retry must not skip a failed layout");
            Ok(())
        }).unwrap();
        assert_eq!(action_for_event(&shared.lock_safe(), &event).unwrap().action_id, "custom.hotkey");
    }

    #[test]
    fn invalid_apply_does_not_disable_a_working_layout() {
        let shared = test_state();
        let prepared = prepare_first(&shared);
        transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        let mut guard = shared.lock_safe();
        let mut dev = guard.devices[0].clone();
        let id = guard.profiles[0].id.clone();
        assert!(prepare_layout(&mut guard, dev.clone(), &id, usize::MAX, true).is_err());
        dev.connected = false;
        assert!(prepare_layout(&mut guard, dev, &id, 0, true).is_err());
        assert!(guard.layout_ready);
    }

    #[test]
    fn live_face_redraw_uses_the_applied_page_after_saved_profiles_change() {
        let shared = test_state();
        shared.lock_safe().profiles[0].pages[0].keys[0].action_id = "audio.volume_up".into();
        let prepared = prepare_first(&shared);
        transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        let mut guard = shared.lock_safe();
        guard.profiles.clear();
        let face = DeckFaceState { volume_percent: Some(37), ..Default::default() };
        let (_, bursts) = face_delta_bursts(&mut guard, &face).unwrap();
        assert!(!bursts.is_empty(), "live faces must still render from the applied snapshot");
    }

    #[test]
    fn shortcut_presets_and_new_obs_actions_are_known_actions() {
        for id in [
            "hotkey.copy",
            "hotkey.snap_left",
            "obs.toggle_virtualcam",
            "obs.toggle_replay",
            "obs.pause_record",
            "obs.studio_transition",
            "obs.set_scene",
            "obs.toggle_mute",
            "media.stop",
        ] {
            assert!(known_action(id), "{id} should be handled by the deck");
        }
        assert!(!known_action("hotkeys.copy"));
    }

    #[test]
    fn a_preset_keeps_its_key_combo_as_the_parameter() {
        let action = binding_action("hotkey.snap_left", "Win+Left", "Snap Left", String::new(), "deck".to_string());
        assert_eq!(action.action_id, "hotkey.snap_left");
        assert_eq!(action.param, "Win+Left");
    }
}
