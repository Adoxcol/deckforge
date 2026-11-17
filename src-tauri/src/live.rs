use super::*;

pub(crate) struct LiveAction {
    pub(crate) action_id: String,
    pub(crate) label: String,
    pub(crate) param: String,
    pub(crate) note: String,
    pub(crate) device_id: String,
    /// The deck key that was pressed, for actions that care (plugins).
    pub(crate) key_index: Option<u8>,
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
            if key.action_id.is_empty() || key.action_id == "none" { return None; }
            Some(LiveAction {
                action_id: key.action_id.clone(),
                label: key.action_label.clone(),
                param: key.action_param.clone(),
                note: format!("Key #{} ({})", evt.control_index + 1, key.title),
                device_id: evt.device_id.clone(),
                key_index: Some(evt.control_index),
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
                key_index: None,
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
                key_index: None,
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
            | "audio.app_volume_up"
            | "audio.app_volume_down"
            | "audio.app_mute"
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
            | "profile.back"
            | "macro.run"
            | "sys.game_mode"
            | "sys.cpu"
            | "sys.ram"
            | "sys.clock"
            | "media.now_playing"
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
            key_index: None,
        }
    } else {
        LiveAction {
            action_id: "custom.hotkey".to_string(),
            label: label.to_string(),
            param: binding.to_string(),
            note,
            device_id,
            key_index: None,
        }
    }
}

pub(crate) fn face_changes(action_id: &str) -> bool {
    action_id.starts_with("audio.app_") || matches!(
        action_id,
        "media.play_pause"
            | "media.now_playing"
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

fn app_audio_label(param: &str, face: &DeckFaceState) -> String {
    if param.trim().is_empty() { return "Choose app".into(); }
    match &face.app_audio {
        None => "Unavailable".into(),
        Some(apps) => apps.get(&param.trim().to_lowercase()).map(|app| app.label()).unwrap_or_else(|| "No session".into()),
    }
}

fn key_level_for(key: &KeyConfig, face: &DeckFaceState) -> Option<u8> {
    if key.action_id.starts_with("audio.app_") {
        let app = face.app_audio.as_ref()?.get(&key.action_param.trim().to_lowercase())?;
        return if app.muted { Some(0) } else if app.min_volume == app.max_volume && !app.mixed_mute { Some(app.min_volume) } else { None };
    }
    key_level(&key.action_id, face)
}

pub(crate) fn key_level(action_id: &str, face: &DeckFaceState) -> Option<u8> {
    match action_id {
        "audio.volume_up" | "audio.volume_down" | "audio.volume_mute" => face.volume_percent,
        "sys.cpu" => face.cpu_percent,
        "sys.ram" => face.ram_percent,
        _ => None,
    }
}

/// Album art for a now-playing key, when the player offers some.
pub(crate) fn key_art<'a>(key: &KeyConfig, face: &'a DeckFaceState) -> Option<&'a [u8]> {
    if key.media.is_none() && key.action_id == "media.now_playing" && !face.media_art.is_empty() {
        Some(face.media_art.as_slice())
    } else if key.media.is_none() && key.action_id.starts_with(crate::plugins::ACTION_PREFIX) {
        face.plugin_keys.get(&key.index).map(|f| f.image.as_slice()).filter(|b| !b.is_empty())
    } else {
        None
    }
}

pub(crate) fn styled_key(key: &KeyConfig, face: &DeckFaceState) -> (String, String) {
    if key.media.is_some() {
        return (key.title.clone(), key.icon_name.clone());
    }
    if key.action_id.starts_with("audio.app_") {
        return (app_audio_label(&key.action_param, face), key.icon_name.clone());
    }
    if key.action_id.starts_with(crate::plugins::ACTION_PREFIX) {
        let title = face.plugin_keys.get(&key.index).and_then(|f| f.title.clone());
        return (title.unwrap_or_else(|| key.title.clone()), key.icon_name.clone());
    }
    match key.action_id.as_str() {
        "sys.cpu" => match face.cpu_percent {
            Some(p) => (format!("CPU {p}%"), key.icon_name.clone()),
            None => (key.title.clone(), key.icon_name.clone()),
        },
        "sys.ram" => match face.ram_percent {
            Some(p) => (format!("RAM {p}%"), key.icon_name.clone()),
            None => (key.title.clone(), key.icon_name.clone()),
        },
        "sys.clock" if !face.clock.is_empty() => (face.clock.clone(), key.icon_name.clone()),
        "media.play_pause" | "media.now_playing" => {
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
    if zone.tap_action_id.starts_with("audio.app_") {
        return app_audio_label(&zone.tap_action_param, face);
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
                level: if key.media.is_some() { None } else { key_level_for(key, face) },
                art: key_art(key, face).map(|bytes| bytes.to_vec()),
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
) -> Option<Result<String, String>> {
    if !matches!(action_id, "brightness.up" | "brightness.down" | "brightness.toggle") {
        return None;
    }
    let (dev, percent) = {
        let guard = shared.lock_safe();
        let Some(dev) = guard.devices.iter().find(|d| d.connected && d.supports_brightness) else {
            return Some(Err("No connected device supports brightness".into()));
        };
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
        (dev.clone(), next)
    };
    if let Err(err) = write_panel_brightness(&dev, percent) { return Some(Err(err)); }
    if let Some(device) = shared.lock_safe().devices.iter_mut().find(|d| d.id == dev.id) { device.brightness = percent; }
    Some(Ok(format!("Backlight set to {percent}%")))
}

/// Streams the saved layout after the deck has accepted the handshake.
/// These panels keep no picture of their own; a later handshake leaves them black until this runs.
/// The device list is read once when the app starts. If the deck was rebooting then (its
/// firmware restarts when the app dies without a heartbeat), the list says "not connected"
/// even though the IO thread opens the deck a few seconds later, and the layout would never
/// be sent. So look again before the first push.
fn ensure_deck_in_state(shared: &SharedState) -> bool {
    for attempt in 0..6u32 {
        let known = |devices: &[DeviceDescriptor]| devices.iter().any(|d| d.connected && d.protocol_family.starts_with("CRT"));
        if known(&shared.lock_safe().devices) {
            return true;
        }
        let (devices, _) = discovery::refresh(shared);
        if known(&devices) {
            deck_note("deck was missing from the device list at start; found it on a later scan");
            return true;
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

pub(crate) fn sync_page_knob_leds(shared: &SharedState, dev: &DeviceDescriptor, page: &ProfilePage) {
    if !page.encoders.iter().any(|e| !e.led_color.trim().is_empty()) {
        return;
    }
    let colors: Vec<String> = (0..4)
        .map(|i| {
            page.encoders
                .iter()
                .find(|e| e.index as usize == i)
                .map(|e| e.led_color.clone())
                .unwrap_or_default()
        })
        .collect();
    let mut diags = Vec::new();
    let _ = send_hardware_knob_leds(dev, &colors, &mut diags);
    if !diags.is_empty() {
        let mut guard = shared.lock_safe();
        guard.diagnostics.extend(diags);
        let excess = guard.diagnostics.len().saturating_sub(80);
        guard.diagnostics.drain(..excess);
    }
}

pub(crate) fn restore_connected_layout(shared: &SharedState) -> Result<(), String> {
    let _display = DISPLAY_WRITE.lock_safe();
    let prepared = prepare_reconnect(&mut shared.lock_safe())?;
    let dev = prepared.dev.clone();
    let page = prepared.applied.page.clone();
    transmit_layout(shared, prepared, write_hid_bursts)?;
    sync_page_knob_leds(shared, &dev, &page);
    Ok(())
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
    let dev = prepared.dev.clone();
    let page = prepared.applied.page.clone();
    transmit_layout(shared, prepared, |dev, bursts| {
        if handshake { write_hid_bursts(dev, bursts) } else { write_animation_frames(dev, bursts) }
    })?;
    sync_page_knob_leds(shared, &dev, &page);
    Ok(())
}

pub(crate) struct PreparedLayout {
    pub(crate) dev: DeviceDescriptor,
    pub(crate) applied: AppliedLayout,
    face: DeckFaceState,
    media: MediaStore,
    frame_cache: AntiFreezeFrameCache,
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
    Ok(PreparedLayout {
        dev,
        applied,
        face: guard.face.clone(),
        // Media clips are immutable Arc snapshots: no image buffers are copied
        // and the library stays available to readers/imports during rendering.
        media: guard.media.clone(),
        frame_cache: guard.frame_cache.clone(),
    })
}

/// DISPLAY_WRITE protects this private cache snapshot until it is published.
fn render_layout_frames(prepared: &mut PreparedLayout) -> (drivers::hal::PublishOutcome, Vec<DiagnosticEntry>) {
    let _timing = performance::timer(performance::Stage::FullRender);
    let (jobs, zones, strip) = plan_screens(&prepared.dev, &prepared.applied.page, &prepared.face);
    let mut diags = Vec::new();
    let outcome = publish_screens(
        &prepared.dev, &jobs, &zones, strip.as_ref(), &prepared.media,
        &mut prepared.frame_cache, &mut diags,
    );
    (outcome, diags)
}

fn render_layout_with(
    shared: &SharedState,
    prepared: &mut PreparedLayout,
    render: impl FnOnce(&mut PreparedLayout) -> (drivers::hal::PublishOutcome, Vec<DiagnosticEntry>),
) -> drivers::hal::PublishOutcome {
    let (outcome, diags) = render(prepared);
    let mut guard = shared.lock_safe();
    guard.frame_cache = std::mem::take(&mut prepared.frame_cache);
    guard.diagnostics.extend(diags);
    let excess = guard.diagnostics.len().saturating_sub(80);
    guard.diagnostics.drain(..excess);
    outcome
}

pub(crate) fn transmit_layout(
    shared: &SharedState,
    mut prepared: PreparedLayout,
    write: impl FnOnce(&DeviceDescriptor, &[HidBurst]) -> Result<(), String>,
) -> Result<SyncKeyResult, String> {
    let outcome = render_layout_with(shared, &mut prepared, render_layout_frames);
    let started = std::time::Instant::now();
    let result = write(&prepared.dev, &outcome.bursts);
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let mut guard = shared.lock_safe();
    guard.frame_cache.finish_transfer(&prepared.dev.id, result.is_ok(),
        outcome.result.bytes_transmitted as u64, elapsed_ms);
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
    guard.playback_slots = outcome.playback;
    guard.playback_device_id = prepared.dev.id;
    guard.applied_to_device_at_ms = Some(now_ms());
    let mut result = outcome.result;
    result.elapsed_ms = elapsed_ms;
    result.status_message = format!("Sent layout to {} in {elapsed_ms}ms", prepared.dev.model_name);
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
    app: &tauri::AppHandle, shared: &SharedState, action: &LiveAction,
) -> ActionExecutionResult {
    let start = std::time::Instant::now();
    let outcome = if action.action_id == "macro.run" {
        Some(start_macro(app, shared, &action.param, &action.label))
    } else if action.action_id == "sys.game_mode" {
        Some(start_game_mode_toggle(app, shared, &action.param))
    } else if action.action_id.starts_with(crate::plugins::ACTION_PREFIX) {
        Some(crate::plugins::key_press(action.key_index))
    } else {
        apply_brightness_action(shared, &action.action_id)
            .or_else(|| apply_navigation(app, shared, &action.action_id, &action.param))
    };
    if let Some(outcome) = outcome { return actions::action_result(&action.action_id, &action.label, start, outcome); }
    let obs = shared.lock_safe().obs.clone();
    let result = execute_key_action_live(&action.action_id, &action.label, &action.param, true, &obs);
    if result.success && face_changes(&action.action_id) { shared.lock_safe().sample_after_ms = now_ms().saturating_add(160); }
    result
}

/// Nudges the key faces so the Game Mode key flips to GAME ON or back quickly.
pub(crate) fn nudge_face_sample(shared: &SharedState) {
    { let mut guard = shared.lock_safe();
        guard.sample_after_ms = now_ms().saturating_add(120);
    }
}

/// Game Mode closes apps one at a time, so it never runs on the input thread. The key's
/// parameter picks the mode by id; an empty one uses the mode chosen in the Game Mode tab.
pub(crate) fn start_game_mode_toggle(app: &tauri::AppHandle, shared: &SharedState, mode: &str) -> Result<String, String> {
    if game_mode::is_busy() {
        return Err("Game Mode is still working".to_string());
    }
    let turning_on = !game_mode::is_active();
    let app = app.clone();
    let shared_for_thread = Arc::clone(shared);
    let mode = mode.trim().to_string();
    std::thread::Builder::new().name("deckforge-game-mode-action".into()).spawn(move || {
        let message = game_mode::toggle(&app, Some(mode.as_str()).filter(|m| !m.is_empty()));
        nudge_face_sample(&shared_for_thread);
        let _ = app.emit("game-mode-message", &message);
    }).map_err(|err| format!("Could not start Game Mode action: {err}"))?;
    nudge_face_sample(shared);
    if turning_on {
        Ok("Game Mode start requested; follow progress in Game Mode".into())
    } else {
        Ok("Game Mode stop requested; follow progress in Game Mode".into())
    }
}

pub(crate) fn apply_navigation(
    app: &tauri::AppHandle,
    shared: &SharedState,
    action_id: &str,
    param: &str,
) -> Option<Result<String, String>> {
    if !matches!(
        action_id,
        "profile.next_page" | "profile.prev_page" | "profile.switch_page" | "profile.switch" | "profile.back"
    ) {
        return None;
    }
    let _display = DISPLAY_WRITE.lock_safe();
    let event = {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        let Some(message) = switch_active_page(&mut guard, action_id, param) else {
            return Some(Err(format!("No matching profile or page for `{param}`")));
        };
        DeckContextEvent {
            profile_id: guard.active_profile_id.clone(),
            page_index: guard.active_page_index,
            message,
        }
    };
    if let Err(err) = push_active_layout_locked(shared, false) {
        return Some(Err(format!("Page switch failed: {err}")));
    }
    let _ = app.emit("deck-context", &event);
    Some(Ok(event.message))
}

pub(crate) fn switch_active_page(guard: &mut ManagedAppState, action_id: &str, param: &str) -> Option<String> {
    let before = (guard.active_profile_id.clone(), guard.active_page_index);
    if action_id == "profile.back" {
        let (profile_id, page_index) = guard.previous_page.take()?;
        let profile = guard.profiles.iter().find(|profile| profile.id == profile_id)?;
        let page_index = page_index.min(profile.pages.len().saturating_sub(1));
        let page_name = profile.pages.get(page_index).map(|page| page.name.clone()).unwrap_or_default();
        guard.active_profile_id = profile_id;
        guard.active_page_index = page_index;
        guard.previous_page = Some(before);
        return Some(format!("Back to {page_name}"));
    }
    let message = switch_active_page_inner(guard, action_id, param)?;
    if (guard.active_profile_id.clone(), guard.active_page_index) != before {
        guard.previous_page = Some(before);
    }
    Some(message)
}

fn switch_active_page_inner(guard: &mut ManagedAppState, action_id: &str, param: &str) -> Option<String> {
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
                _ => resolve_page_index(guard, param)?,
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

/// Windows media/COM calls may block. Never hold app state while sampling them.
pub(crate) fn sample_face(shared: &SharedState) -> bool {
    let needs_apps = {
        let guard = shared.lock_safe();
        let uses = |page: &ProfilePage| {
            page.keys.iter().any(|k| k.action_id.starts_with("audio.app_"))
                || page.touch_zones.iter().any(|z| z.tap_action_id.starts_with("audio.app_"))
                || page.encoders.iter().any(|e| [&e.cw_action, &e.ccw_action, &e.push_action].iter().any(|a| a.starts_with("audio.app_")))
        };
        guard.profiles.iter().flat_map(|p| &p.pages).any(uses)
            || guard.applied_layout.as_ref().is_some_and(|a| uses(&a.page))
    };
    sample_face_with(shared, |previous| read_face(previous, needs_apps))
}

fn sample_face_with(
    shared: &SharedState,
    sample: impl FnOnce(&DeckFaceState) -> DeckFaceState,
) -> bool {
    let (previous, refresh_requested) = {
        let mut guard = shared.lock_safe();
        let refresh_requested = guard.sample_after_ms > 0;
        guard.sample_after_ms = 0;
        (guard.face.clone(), refresh_requested)
    };
    let next = sample(&previous);
    let mut guard = shared.lock_safe();
    let changed = next != guard.face;
    if changed {
        guard.face = next;
    }
    // Requests arriving during the OS query remain scheduled for the next pass.
    changed || refresh_requested
}

fn read_face(previous: &DeckFaceState, needs_apps: bool) -> DeckFaceState {
    let _timing = performance::timer(performance::Stage::Media);
    let (playing, title) = read_media_face();
    let audio = read_audio_face();
    let stats = crate::actions::read_quick_stats();
    let media_art = if previous.media_title == title && !previous.media_art.is_empty() {
        previous.media_art.clone()
    } else if playing.is_some() {
        crate::actions::read_media_art()
    } else {
        Vec::new()
    };
    DeckFaceState {
        media_playing: playing,
        media_title: title,
        mic_muted: audio.mic_muted,
        speaker_muted: audio.speaker_muted,
        volume_percent: audio.volume_percent,
        app_audio: if needs_apps { actions::app_audio::list().ok().map(|apps| apps.into_iter().map(|app| (app.exe.clone(), app)).collect()) } else { None },
        game_mode: Some(game_mode::is_active()),
        game_mode_busy: game_mode::is_busy(),
        cpu_percent: stats.cpu_percent,
        ram_percent: stats.ram_percent,
        clock: stats.clock,
        media_art,
        plugin_keys: crate::plugins::current_faces(),
    }
}

pub(crate) fn push_face_deltas(shared: &SharedState, app: &tauri::AppHandle) {
    let _display = DISPLAY_WRITE.lock_safe();
    let face = shared.lock_safe().face.clone();
    let _ = app.emit("deck-face", &face);
    let prepared = face_delta_bursts(shared, &face);
    let Some((dev, bursts)) = prepared else {
        return;
    };
    if bursts.is_empty() {
        return;
    }
    let _ = transmit_cached_frames(shared, &dev, &bursts, write_animation_frames);
}

/// Caller holds DISPLAY_WRITE from planning through completion. Background
/// redraws retain their bindings, but failed frames must never enter the cache.
pub(crate) fn transmit_cached_frames(
    shared: &SharedState,
    dev: &DeviceDescriptor,
    bursts: &[HidBurst],
    write: impl FnOnce(&DeviceDescriptor, &[HidBurst]) -> Result<(), String>,
) -> Result<(), String> {
    let started = std::time::Instant::now();
    let result = write(dev, bursts);
    let bytes = bursts.iter().flat_map(|b| &b.reports).map(|r| r.len() as u64).sum();
    let mut guard = shared.lock_safe();
    guard.frame_cache.finish_transfer(&dev.id, result.is_ok(), bytes, started.elapsed().as_millis() as u64);
    if let Err(err) = &result {
        // Retry even if the OS face values do not change again.
        guard.sample_after_ms = now_ms().saturating_add(500);
        guard.diagnostics.push(DiagnosticEntry {
            id: format!("diag-frame-failed-{}", now_ms()), timestamp_ms: now_ms(),
            level: "warning".into(), device_id: dev.id.clone(), stage: "FRAME_TRANSFER".into(),
            message: format!("Frame transfer failed; will retry: {err}"), packet_hex: None,
        });
        let excess = guard.diagnostics.len().saturating_sub(80);
        guard.diagnostics.drain(..excess);
    }
    result
}

struct RenderedFaceFrame {
    slot: String,
    hash_payload: Vec<u8>,
    jpeg: Vec<u8>,
    image_slot: u8,
}

/// Caller holds DISPLAY_WRITE until transmission finishes, keeping the applied
/// page and frame-cache batch stable while expensive rendering releases state.
pub(crate) fn face_delta_bursts(shared: &SharedState, face: &DeckFaceState) -> Option<(DeviceDescriptor, Vec<HidBurst>)> {
    face_delta_bursts_with(shared, face, render_face_frames)
}

fn face_delta_bursts_with(
    shared: &SharedState,
    face: &DeckFaceState,
    render: impl FnOnce(&DeviceDescriptor, &ProfilePage, &DeckFaceState) -> Vec<RenderedFaceFrame>,
) -> Option<(DeviceDescriptor, Vec<HidBurst>)> {
    let (dev, page) = {
        let guard = shared.lock_safe();
        if !guard.layout_ready { return None; }
        let dev = guard.devices.iter()
            .find(|d| d.connected && d.protocol_family.starts_with("CRT"))?.clone();
        (dev, guard.applied_layout.as_ref()?.page.clone())
    };
    let frames = render(&dev, &page, face);
    let pending = {
        let mut guard = shared.lock_safe();
        // Pause/disconnect may happen while rendering even though layout writes
        // are serialized by DISPLAY_WRITE. Do not stage those obsolete frames.
        if !guard.layout_ready || !guard.devices.iter().any(|d| d.id == dev.id && d.connected) {
            return None;
        }
        frames.into_iter().filter(|frame| {
            guard.frame_cache.should_transmit(&dev.id, &frame.slot, &frame.hash_payload).is_some()
        }).collect::<Vec<_>>()
    };
    let bursts = pending.into_iter().map(|frame| HidBurst {
        reports: build_key_image_packets(frame.image_slot, &frame.jpeg)
            .into_iter().map(|report| report.to_vec()).collect(),
    }).collect();
    Some((dev, bursts))
}

fn render_face_frames(dev: &DeviceDescriptor, page: &ProfilePage, face: &DeckFaceState) -> Vec<RenderedFaceFrame> {
    let _timing = performance::timer(performance::Stage::FaceRender);
    let strip_covers_zones = page.strip_media.is_some();
    let mut bursts = Vec::new();
    let key_count = dev.keys.count as usize;
    for key in page.keys.iter().take(key_count) {
        if key.media.is_some() {
            continue;
        }
        let level = key_level_for(key, face);
        let (title, icon) = styled_key(key, face);
        let art = key_art(key, face);
        if title == key.title && icon == key.icon_name && level.is_none() && art.is_none() {
            continue;
        }
        let art_jpeg = art.and_then(|bytes| {
            crate::media::render_art_key(
                bytes,
                dev.keys.pixel_width,
                dev.keys.pixel_height,
                dev.keys.rotation,
                if key.show_title { Some(title.as_str()) } else { None },
            )
        });
        let jpeg = if let Some(jpeg) = art_jpeg { jpeg } else { encode_procedural_key_jpeg(
            dev.keys.pixel_width,
            dev.keys.pixel_height,
            &key.accent_color,
            &key.bg_color,
            &icon,
            if key.show_title { title.as_str() } else { "" },
            key.index,
            dev.keys.rotation,
            level,
        ) };
        let mut hash = jpeg.clone();
        hash.extend_from_slice(title.as_bytes());
        hash.extend_from_slice(icon.as_bytes());
        let slot = format!("key-{}", key.index);
        bursts.push(RenderedFaceFrame {
            slot, hash_payload: hash, jpeg,
            image_slot: key_image_slot(dev, key.index),
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
            bursts.push(RenderedFaceFrame {
                slot, hash_payload: jpeg.clone(), jpeg,
                image_slot: touch_zone_image_slot(dev, zone.index),
            });
        }
    }
    bursts
}

/// The profile and page to show for the app in front, or None to stay put.
/// The first enabled rule by priority wins. A rule naming a page switches even inside the
/// profile already showing; a rule without one (or whose page was deleted) opens page 1,
/// and only when that profile is not already showing.
pub(crate) fn auto_switch_target(
    rules: &[AutoSwitchRule],
    profiles: &[Profile],
    active_profile_id: &str,
    active_page_index: usize,
    exe: &str,
    title: &str,
    is_game: bool,
) -> Option<(String, usize)> {
    let mut rules: Vec<&AutoSwitchRule> = rules.iter().collect();
    // A rule for one particular app beats the catch-all game rule.
    rules.sort_by_key(|rule| (is_any_game(rule), rule.priority));
    let rule = rules.into_iter().find(|rule| rule_matches(rule, exe, title, is_game))?;
    let profile = profiles.iter().find(|profile| profile.id == rule.target_profile_id)?;
    let page = (!rule.target_page_id.is_empty())
        .then(|| profile.pages.iter().position(|page| page.id == rule.target_page_id))
        .flatten();
    let same_profile = rule.target_profile_id == active_profile_id;
    match page {
        Some(index) if same_profile && index == active_page_index => None,
        Some(index) => Some((rule.target_profile_id.clone(), index)),
        None if same_profile => None,
        None => Some((rule.target_profile_id.clone(), 0)),
    }
}

/// Process name of the rule that matches any full-screen game instead of one program.
pub(crate) const ANY_GAME: &str = "@game";

fn is_any_game(rule: &AutoSwitchRule) -> bool {
    rule.process_name.trim().eq_ignore_ascii_case(ANY_GAME)
}

pub(crate) fn rule_matches(rule: &AutoSwitchRule, exe: &str, title: &str, is_game: bool) -> bool {
    if !rule.enabled {
        return false;
    }
    if is_any_game(rule) {
        return is_game;
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
    pub(super) fn test_state() -> SharedState {
        let mut devices = builtin_device_catalog();
        devices[0].connected = true;
        Arc::new(Mutex::new(ManagedAppState {
            devices,
            profiles: db::default_profiles(),
            active_profile_id: "profile-streaming".into(),
            active_page_index: 0,
            previous_page: None,
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

    #[test]
    fn slow_face_sample_leaves_state_available_and_preserves_new_refresh() {
        use std::sync::mpsc;
        let shared = test_state();
        let worker_state = Arc::clone(&shared);
        let (started_tx, started_rx) = mpsc::channel();
        let (resume_tx, resume_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || sample_face_with(&worker_state, |previous| {
            started_tx.send(()).unwrap();
            resume_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let mut next = previous.clone();
            next.media_title = "New track".into();
            next
        }));
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let accessible = if let Ok(mut guard) = shared.try_lock() {
            guard.sample_after_ms = 123;
            true
        } else {
            false
        };
        resume_tx.send(()).unwrap();
        let changed = worker.join().unwrap();
        assert!(accessible, "UI and key commands must access state while Windows is stalled");
        assert!(changed);
        let guard = shared.lock_safe();
        assert_eq!(guard.sample_after_ms, 123);
        assert_eq!(guard.face.media_title, "New track");
    }

    #[test]
    fn unchanged_face_only_redraws_when_refresh_was_requested() {
        let shared = test_state();
        assert!(!sample_face_with(&shared, Clone::clone));
        shared.lock_safe().sample_after_ms = 123;
        assert!(sample_face_with(&shared, Clone::clone));
        assert_eq!(shared.lock_safe().sample_after_ms, 0);
        assert!(!sample_face_with(&shared, Clone::clone));
    }

    fn prepare_first(shared: &SharedState) -> PreparedLayout {
        let mut guard = shared.lock_safe();
        let dev = guard.devices[0].clone();
        let id = guard.profiles[0].id.clone();
        prepare_layout(&mut guard, dev, &id, 0, true).unwrap()
    }

    #[test]
    fn clear_key_preserves_applied_binding_until_successful_apply_and_survives_recovery() {
        let shared = test_state();
        transmit_layout(&shared, prepare_first(&shared), |_, _| Ok(())).unwrap();
        let event = press(InputControlKind::LcdKey, 0);
        let original = shared.lock_safe().applied_layout.clone().unwrap();
        {
            // Saving an editor clear alone must not blank the physical image or change its binding.
            let mut guard = shared.lock_safe();
            let key = &mut guard.profiles[0].pages[0].keys[0];
            key.title = "Empty".into(); key.action_id = "none".into(); key.action_param.clear(); key.media = None;
            assert_eq!(action_for_event(&guard, &event).unwrap().action_id, original.page.keys[0].action_id);
            assert_eq!(guard.applied_layout.as_ref().unwrap().page.keys[0].title, original.page.keys[0].title);
        }
        let prepared = prepare_first(&shared);
        assert!(transmit_layout(&shared, prepared, |_, _| Err("partial clear transfer".into())).is_err());
        assert!(action_for_event(&shared.lock_safe(), &event).is_none(), "failed transfer must suppress dispatch");
        assert_eq!(shared.lock_safe().applied_layout.as_ref().unwrap().page.keys[0].title, original.page.keys[0].title);
        let prepared = prepare_first(&shared);
        assert_eq!(prepared.applied.page.keys[0].action_id, "none");
        transmit_layout(&shared, prepared, |_, bursts| { assert!(!bursts.is_empty()); Ok(()) }).unwrap();
        {
            let guard = shared.lock_safe();
            assert!(guard.layout_ready);
            assert!(action_for_event(&guard, &event).is_none());
            assert!(action_for_event(&guard, &press(InputControlKind::LcdKey, 1)).is_some());
            assert_eq!(guard.applied_layout.as_ref().unwrap().page.keys[0].title, "Empty");
        }
        let recovery = prepare_reconnect(&mut shared.lock_safe()).unwrap();
        assert_eq!(recovery.applied.page.keys[0].action_id, "none");
        transmit_layout(&shared, recovery, |_, _| Ok(())).unwrap();
        assert!(action_for_event(&shared.lock_safe(), &event).is_none());
    }

    #[test]
    fn missing_navigation_targets_and_disconnected_brightness_report_failure() {
        let shared = test_state();
        let mut guard = shared.lock_safe();
        let before = (guard.active_profile_id.clone(), guard.active_page_index);
        assert!(switch_active_page(&mut guard, "profile.switch_page", "nonexistent-page").is_none());
        assert!(switch_active_page(&mut guard, "profile.switch", "nonexistent-profile").is_none());
        assert_eq!((guard.active_profile_id.clone(), guard.active_page_index), before);
        for dev in &mut guard.devices { dev.connected = false; }
        let brightness = guard.devices[0].brightness;
        drop(guard);
        assert!(apply_brightness_action(&shared, "brightness.up").unwrap().is_err());
        assert_eq!(shared.lock_safe().devices[0].brightness, brightness);
    }

    #[test]
    fn full_layout_stats_commit_only_after_successful_transport() {
        let shared = test_state();
        let prepared = prepare_first(&shared);
        assert_eq!(shared.lock_safe().frame_cache.stats.frames_transmitted, 0);
        assert!(transmit_layout(&shared, prepared, |_, _| Err("partial write".into())).is_err());
        assert_eq!(shared.lock_safe().frame_cache.stats.bytes_transmitted, 0);
        let prepared = prepare_first(&shared);
        let mut bytes = 0;
        let mut count = 0;
        transmit_layout(&shared, prepared, |_, bursts| {
            bytes = bursts.iter().flat_map(|b| &b.reports).map(|r| r.len() as u64).sum();
            count = bursts.len() as u64;
            Ok(())
        }).unwrap();
        let guard = shared.lock_safe();
        assert_eq!(guard.frame_cache.stats.bytes_transmitted, bytes);
        assert_eq!(guard.frame_cache.stats.frames_transmitted, count);
    }

    #[test]
    fn face_render_releases_state_and_discards_frames_after_pause() {
        let shared = test_state();
        let prepared = prepare_first(&shared);
        transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        let evaluated = shared.lock_safe().frame_cache.stats.frames_evaluated;
        let result = face_delta_bursts_with(&shared, &DeckFaceState::default(), |_, _, _| {
            let mut guard = shared.try_lock().expect("rendering must leave UI and key state accessible");
            guard.layout_ready = false;
            vec![RenderedFaceFrame {
                slot: "key-0".into(), hash_payload: vec![1], jpeg: vec![1], image_slot: 1,
            }]
        });
        assert!(result.is_none());
        assert_eq!(shared.lock_safe().frame_cache.stats.frames_evaluated, evaluated);
    }

    #[test]
    fn unchanged_live_face_is_retryable_after_a_failed_redraw() {
        let shared = test_state();
        shared.lock_safe().profiles[0].pages[0].keys[0].action_id = "audio.volume_up".into();
        let prepared = prepare_first(&shared);
        transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        let face = DeckFaceState { volume_percent: Some(73), ..Default::default() };
        let count = shared.lock_safe().frame_cache.stats.frames_transmitted;
        let (dev, bursts) = face_delta_bursts(&shared, &face).unwrap();
        assert!(!bursts.is_empty());
        assert!(transmit_cached_frames(&shared, &dev, &bursts, |_, _| Err("write failed".into())).is_err());
        assert_eq!(shared.lock_safe().frame_cache.stats.frames_transmitted, count);
        assert!(shared.lock_safe().sample_after_ms > 0);
        let (dev, retry) = face_delta_bursts(&shared, &face).unwrap();
        assert!(!retry.is_empty());
        transmit_cached_frames(&shared, &dev, &retry, |_, _| Ok(())).unwrap();
        let (_, unchanged) = face_delta_bursts(&shared, &face).unwrap();
        assert!(unchanged.is_empty());
    }

    static ANIMATION_TEST: Mutex<()> = Mutex::new(());

    #[test]
    fn animations_prioritize_overdue_slots_and_build_without_state_lock() {
        let _test = ANIMATION_TEST.lock_safe();
        let shared = test_state();
        transmit_layout(&shared, prepare_first(&shared), |_, _| Ok(())).unwrap();
        shared.lock_safe().playback_slots = (0..6).map(|index| PlaybackSlot {
            slot_id: format!("key-{index}"), target: PlaybackTarget::Key(index),
            jpegs: vec![vec![index, 1], vec![index, 2]], delays_ms: vec![80, 80],
            loop_playback: true, frame_index: 0,
            next_due_ms: if index < 3 { 1 } else { 0 },
        }).collect();
        for expected in [vec![3, 4, 5], vec![0, 1, 2]] {
            let mut built = false;
            workers::animation_tick_with(&shared, |dev, frames| {
                built = true;
                let guard = shared.try_lock().expect("packet assembly must leave UI state accessible");
                assert!(guard.layout_ready);
                drop(guard);
                assert!(DISPLAY_WRITE.try_lock().is_err(), "display ordering must be retained");
                let indices: Vec<_> = frames.iter().map(|(target, _)| match target {
                    PlaybackTarget::Key(index) => *index,
                    _ => panic!("unexpected target"),
                }).collect();
                assert_eq!(indices, expected);
                workers::build_animation_bursts(dev, frames)
            }, |_, bursts| { assert_eq!(bursts.len(), 3); Ok(()) }).unwrap();
            assert!(built);
        }
    }

    #[test]
    fn non_looping_animation_retries_its_final_frame_after_write_failure() {
        let _test = ANIMATION_TEST.lock_safe();
        let shared = test_state();
        let prepared = prepare_first(&shared);
        transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        {
            let mut guard = shared.lock_safe();
            guard.playback_slots = vec![PlaybackSlot {
                slot_id: "key-0".into(), target: PlaybackTarget::Key(0),
                jpegs: vec![b"first".to_vec(), b"last".to_vec()],
                delays_ms: vec![80, 80], loop_playback: false, frame_index: 1, next_due_ms: 0,
            }];
        }
        let before = shared.lock_safe().frame_cache.stats.frames_transmitted;
        assert!(workers::animation_tick(&shared, |_, _| Err("USB dropped".into())).is_err());
        {
            let mut guard = shared.lock_safe();
            assert_eq!(guard.playback_slots[0].frame_index, 1);
            assert_ne!(guard.playback_slots[0].next_due_ms, u64::MAX);
            assert_eq!(guard.frame_cache.stats.frames_transmitted, before);
            guard.playback_slots[0].next_due_ms = 0;
        }
        workers::animation_tick(&shared, |_, bursts| {
            assert_eq!(bursts.len(), 1);
            Ok(())
        }).unwrap();
        let guard = shared.lock_safe();
        assert_eq!(guard.playback_slots[0].next_due_ms, u64::MAX);
        assert_eq!(guard.frame_cache.stats.frames_transmitted, before + 1);
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
    fn full_layout_render_keeps_state_accessible_and_uses_its_snapshot() {
        let shared = test_state();
        let mut prepared = prepare_first(&shared);
        let title = prepared.applied.page.keys[0].title.clone();
        let outcome = render_layout_with(&shared, &mut prepared, |plan| {
            {
                let mut guard = shared.try_lock().expect("full rendering must not lock UI state");
                assert!(!guard.layout_ready, "key dispatch stays disabled until transfer succeeds");
                guard.profiles[0].pages[0].keys[0].title = "Edited during rendering".into();
            }
            assert_eq!(plan.applied.page.keys[0].title, title);
            render_layout_frames(plan)
        });
        assert!(!outcome.bursts.is_empty());
        let guard = shared.lock_safe();
        assert_eq!(guard.profiles[0].pages[0].keys[0].title, "Edited during rendering");
        assert!(!guard.layout_ready);
        assert_eq!(guard.frame_cache.stats.frames_transmitted, 0);
    }

    #[test]
    fn usb_scan_releases_state_and_preserves_concurrent_settings_and_diagnostics() {
        let shared = test_state();
        shared.lock_safe().devices[0].connected = false;
        let (devices, newly_connected) = discovery::refresh_with(&shared, |devices, diagnostics| {
            let mut guard = shared.try_lock().expect("USB enumeration must not hold app state");
            guard.devices[0].brightness = 23;
            guard.diagnostics.push(DiagnosticEntry {
                id: "during-scan".into(), timestamp_ms: 1, level: "info".into(),
                device_id: "system".into(), stage: "TEST".into(), message: "Concurrent event".into(), packet_hex: None,
            });
            devices[0].connected = true;
            devices[0].hid_path = Some("discovered-path".into());
            for index in 0..79 {
                diagnostics.push(DiagnosticEntry {
                    id: format!("scan-{index}"), timestamp_ms: 2, level: "info".into(),
                    device_id: "system".into(), stage: "USB_SCAN".into(), message: "Scan event".into(), packet_hex: None,
                });
            }
        });
        assert!(newly_connected);
        assert_eq!(devices[0].brightness, 23);
        assert_eq!(devices[0].hid_path.as_deref(), Some("discovered-path"));
        let guard = shared.lock_safe();
        assert_eq!(guard.diagnostics.len(), 80);
        assert!(guard.diagnostics.iter().any(|d| d.id == "during-scan"));
        drop(guard);
        let (_, newly_connected) = discovery::refresh_with(&shared, |_, _| {});
        assert!(!newly_connected, "unchanged scans must not request another full layout");
    }

    #[test]
    fn profile_save_releases_state_and_only_publishes_successful_writes() {
        let shared = test_state();
        let before = serde_json::to_value(&shared.lock_safe().profiles).unwrap();
        let mut profiles = shared.lock_safe().profiles.clone();
        profiles[0].name = "Saved revision".into();
        let failed = commands::save_profiles_with(&shared, profiles.clone(), |_| {
            let guard = shared.try_lock().expect("disk writes must leave app state accessible");
            assert_eq!(serde_json::to_value(&guard.profiles).unwrap(), before);
            Err("disk full".into())
        });
        assert!(failed.is_err());
        assert_eq!(serde_json::to_value(&shared.lock_safe().profiles).unwrap(), before);
        assert_eq!(shared.lock_safe().saved_locally_at_ms, 0);
        let saved_at = commands::save_profiles_with(&shared, profiles, |written| {
            let guard = shared.try_lock().expect("disk writes must leave app state accessible");
            assert_eq!(serde_json::to_value(&guard.profiles).unwrap(), before);
            assert_eq!(written[0].name, "Saved revision");
            Ok(())
        }).unwrap();
        let guard = shared.lock_safe();
        assert_eq!(guard.profiles[0].name, "Saved revision");
        assert_eq!(guard.saved_locally_at_ms, saved_at);
    }

    #[test]
    fn legacy_parsing_releases_state_and_returns_only_unsaved_candidates() {
        let shared = test_state();
        let mut candidates = db::default_profiles();
        candidates.truncate(1);
        candidates[0].id = "imported-candidate".into();
        let result = commands::import_legacy_with(&shared, || {
            let mut guard = shared.try_lock().expect("legacy file parsing must not lock app state");
            guard.profiles[0].name = "Newer saved edit".into();
            candidates
        });
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "imported-candidate");
        let guard = shared.lock_safe();
        assert_eq!(guard.profiles[0].name, "Newer saved edit");
        assert!(!guard.profiles.iter().any(|p| p.id == "imported-candidate"));
        assert_eq!(guard.saved_locally_at_ms, 0);
    }

    #[test]
    fn per_app_audio_actions_are_valid_knob_bindings_and_require_a_target() {
        for id in ["audio.app_volume_up", "audio.app_volume_down", "audio.app_mute"] {
            assert!(known_action(id));
            assert!(actions::app_audio::execute(id, "").is_err());
        }
    }

    #[test]
    fn app_audio_faces_use_the_target_session_and_clear_missing_values() {
        let shared = test_state();
        let mut key = shared.lock_safe().profiles[0].pages[0].keys[0].clone();
        key.action_id = "audio.app_volume_up".into();
        key.action_param = " Spotify.EXE ".into();
        key.media = None;
        let app = actions::app_audio::AudioApp {
            exe: "spotify.exe".into(), sessions: 1, min_volume: 25, max_volume: 25, muted: false, mixed_mute: false,
        };
        let mut face = DeckFaceState {
            volume_percent: Some(99), app_audio: Some([(app.exe.clone(), app)].into_iter().collect()), ..Default::default()
        };
        assert_eq!(styled_key(&key, &face).0, "25%");
        assert_eq!(key_level_for(&key, &face), Some(25));
        face.app_audio.as_mut().unwrap().clear();
        assert_eq!(styled_key(&key, &face).0, "No session");
        assert_eq!(key_level_for(&key, &face), None);
        face.app_audio = None;
        assert_eq!(styled_key(&key, &face).0, "Unavailable");
        assert!(face_changes(&key.action_id));
    }

    #[test]
    fn media_staging_releases_state_and_merges_only_successful_new_clips() {
        let fixture = TestLayoutStore::new();
        let dir = fixture.0.parent().unwrap().join("media");
        std::fs::create_dir(&dir).unwrap();
        let shared = test_state();
        shared.lock_safe().media = MediaStore::load_from(dir.clone());
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2).write_to(&mut png, image::ImageFormat::Png).unwrap();
        let bytes = png.into_inner();
        let preview = commands::stage_media_with(&shared, |staged| {
            assert!(staged.previews().is_empty());
            {
                let mut guard = shared.try_lock().expect("media decoding/writes must not lock app state");
                guard.media.stage_bytes("concurrent", "other.png", &bytes)?;
            }
            staged.stage_bytes("upload", "upload.png", &bytes)
        }).unwrap();
        assert_eq!(preview.id, "upload");
        assert_eq!(shared.lock_safe().media.previews().len(), 2);
        let failed = commands::stage_media_with(&shared, |staged| {
            assert!(shared.try_lock().is_ok());
            staged.stage_bytes("upload", "broken.png", b"invalid image")
        });
        assert!(failed.is_err());
        let library = shared.lock_safe().media.previews();
        assert_eq!(library.len(), 2);
        assert_eq!(library.iter().find(|p| p.id == "upload").unwrap().file_name, "upload.png");
        assert_eq!(MediaStore::load_from(dir.clone()).previews().len(), 2);
        // This dedicated fixture contains only files written by this test.
        for file in std::fs::read_dir(&dir).unwrap() { std::fs::remove_file(file.unwrap().path()).unwrap(); }
        std::fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn settings_write_does_not_lock_state_or_publish_failures() {
        let shared = test_state();
        let original = shared.lock_safe().obs.url.clone();
        let failed = commands::save_setting_with::<ObsSettings>(&shared, || {
            assert!(shared.try_lock().is_ok(), "settings disk I/O must release state");
            Err("disk full".into())
        }, |_, _| panic!("failed write must not publish"));
        assert!(failed.is_err());
        assert_eq!(shared.lock_safe().obs.url, original);
        commands::save_setting_with(&shared, || {
            let mut guard = shared.try_lock().expect("settings disk I/O must release state");
            assert_eq!(guard.obs.url, original);
            guard.devices[0].brightness = 37;
            Ok(ObsSettings { url: "ws://localhost:4456".into(), ..Default::default() })
        }, |guard, value| guard.obs = value).unwrap();
        let guard = shared.lock_safe();
        assert_eq!(guard.obs.url, "ws://localhost:4456");
        assert_eq!(guard.devices[0].brightness, 37);
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
    fn stats_clock_and_album_art_keys_follow_the_live_face() {
        let key = |action: &str| KeyConfig {
            index: 0, title: "Key".into(), subtitle: String::new(), icon_name: "Cpu".into(),
            action_id: action.into(), action_label: String::new(), action_category: String::new(),
            action_param: String::new(), accent_color: "#fff".into(), bg_color: "#000".into(),
            show_title: true, title_size: "Medium".into(), background_style: "Gradient".into(),
            corner_radius: 8, media: None,
        };
        let face = DeckFaceState {
            cpu_percent: Some(42), ram_percent: Some(61), clock: "09:05".into(),
            media_art: vec![1, 2, 3], ..Default::default()
        };
        assert_eq!(styled_key(&key("sys.cpu"), &face).0, "CPU 42%");
        assert_eq!(key_level("sys.cpu", &face), Some(42));
        assert_eq!(styled_key(&key("sys.ram"), &face).0, "RAM 61%");
        assert_eq!(key_level("sys.ram", &face), Some(61));
        assert_eq!(styled_key(&key("sys.clock"), &face).0, "09:05");
        assert_eq!(key_art(&key("media.now_playing"), &face), Some(&[1u8, 2, 3][..]));
        assert_eq!(key_art(&key("media.play_pause"), &face), None);
        // Before the first sample the key keeps its own title.
        assert_eq!(styled_key(&key("sys.cpu"), &DeckFaceState::default()).0, "Key");
        assert!(known_action("sys.clock") && known_action("media.now_playing"));
    }

    #[test]
    fn live_face_redraw_uses_the_applied_page_after_saved_profiles_change() {
        let shared = test_state();
        shared.lock_safe().profiles[0].pages[0].keys[0].action_id = "audio.volume_up".into();
        let prepared = prepare_first(&shared);
        transmit_layout(&shared, prepared, |_, _| Ok(())).unwrap();
        let mut guard = shared.lock_safe();
        guard.profiles.clear();
        drop(guard);
        let face = DeckFaceState { volume_percent: Some(37), ..Default::default() };
        let (_, bursts) = face_delta_bursts(&shared, &face).unwrap();
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

#[cfg(test)]
mod auto_switch_tests {
    use super::auto_switch_target;
    use crate::db::{AutoSwitchRule, Profile, ProfilePage};

    fn page(id: &str) -> ProfilePage {
        ProfilePage { id: id.into(), name: id.into(), keys: vec![], touch_zones: vec![], encoders: vec![], strip_media: None }
    }

    fn profile(id: &str, pages: &[&str]) -> Profile {
        Profile {
            id: id.into(),
            name: id.into(),
            icon: String::new(),
            color_strip: String::new(),
            target_device_id: String::new(),
            pages: pages.iter().map(|p| page(p)).collect(),
            last_modified_ms: 0,
        }
    }

    fn rule(exe: &str, profile: &str, page: &str, priority: u8) -> AutoSwitchRule {
        AutoSwitchRule {
            id: format!("{exe}-{priority}"),
            process_name: exe.into(),
            window_title_contains: String::new(),
            target_profile_id: profile.into(),
            target_page_id: page.into(),
            priority,
            enabled: true,
        }
    }

    fn profiles() -> Vec<Profile> {
        vec![profile("stream", &["main", "scenes"]), profile("edit", &["timeline"])]
    }

    #[test]
    fn page_rule_switches_inside_the_profile_already_showing() {
        let rules = [rule("obs64.exe", "stream", "scenes", 1)];
        assert_eq!(auto_switch_target(&rules, &profiles(), "stream", 0, "obs64.exe", "", false), Some(("stream".into(), 1)));
    }

    #[test]
    fn page_rule_stays_put_when_that_page_is_showing() {
        let rules = [rule("obs64.exe", "stream", "scenes", 1)];
        assert_eq!(auto_switch_target(&rules, &profiles(), "stream", 1, "obs64.exe", "", false), None);
    }

    #[test]
    fn profile_rule_does_not_pull_back_to_page_one() {
        let rules = [rule("obs64.exe", "stream", "", 1)];
        assert_eq!(auto_switch_target(&rules, &profiles(), "stream", 1, "obs64.exe", "", false), None);
        assert_eq!(auto_switch_target(&rules, &profiles(), "edit", 0, "obs64.exe", "", false), Some(("stream".into(), 0)));
    }

    #[test]
    fn deleted_page_falls_back_to_page_one() {
        let rules = [rule("resolve.exe", "edit", "gone", 1)];
        assert_eq!(auto_switch_target(&rules, &profiles(), "stream", 1, "resolve.exe", "", false), Some(("edit".into(), 0)));
    }

    #[test]
    fn missing_profile_disabled_rule_and_other_apps_do_nothing() {
        let mut off = rule("obs64.exe", "stream", "scenes", 1);
        off.enabled = false;
        assert_eq!(auto_switch_target(&[off], &profiles(), "edit", 0, "obs64.exe", "", false), None);
        let rules = [rule("obs64.exe", "nope", "", 1)];
        assert_eq!(auto_switch_target(&rules, &profiles(), "edit", 0, "obs64.exe", "", false), None);
        assert_eq!(auto_switch_target(&rules, &profiles(), "edit", 0, "code.exe", "", false), None);
    }

    #[test]
    fn lowest_priority_number_wins() {
        let rules = [rule("obs64.exe", "edit", "", 2), rule("obs64", "stream", "scenes", 1)];
        assert_eq!(auto_switch_target(&rules, &profiles(), "edit", 0, "OBS64.EXE", "", false), Some(("stream".into(), 1)));
    }
}

#[cfg(test)]
mod back_tests {
    use super::*;

    #[test]
    fn back_returns_to_the_page_shown_before() {
        let shared = super::live_tests::test_state();
        let mut guard = shared.lock().unwrap();
        assert!(switch_active_page(&mut guard, "profile.back", "").is_none(), "nothing to go back to yet");
        switch_active_page(&mut guard, "profile.next_page", "").unwrap();
        assert_eq!(guard.active_page_index, 1);
        switch_active_page(&mut guard, "profile.back", "").unwrap();
        assert_eq!(guard.active_page_index, 0);
        // Back again toggles forward, like a folder key and its back key.
        switch_active_page(&mut guard, "profile.back", "").unwrap();
        assert_eq!(guard.active_page_index, 1);
    }
}

#[cfg(test)]
mod any_game_tests {
    use super::{auto_switch_target, ANY_GAME};
    use crate::db::{AutoSwitchRule, Profile, ProfilePage};

    fn profile(id: &str) -> Profile {
        Profile {
            id: id.into(), name: id.into(), icon: String::new(), color_strip: String::new(),
            target_device_id: String::new(), last_modified_ms: 0,
            pages: vec![ProfilePage { id: format!("{id}-p"), name: id.into(), keys: vec![], touch_zones: vec![], encoders: vec![], strip_media: None }],
        }
    }

    fn rule(exe: &str, profile: &str, priority: u8) -> AutoSwitchRule {
        AutoSwitchRule {
            id: exe.into(), process_name: exe.into(), window_title_contains: String::new(),
            target_profile_id: profile.into(), target_page_id: String::new(), priority, enabled: true,
        }
    }

    #[test]
    fn any_game_rule_opens_for_games_only_and_loses_to_an_app_rule() {
        let profiles = [profile("work"), profile("gaming"), profile("valorant")];
        let rules = [rule(ANY_GAME, "gaming", 1), rule("valorant.exe", "valorant", 9)];
        assert_eq!(auto_switch_target(&rules, &profiles, "work", 0, "eldenring.exe", "", true), Some(("gaming".into(), 0)));
        assert_eq!(auto_switch_target(&rules, &profiles, "work", 0, "chrome.exe", "", false), None);
        // The game's own rule wins even though its priority number is higher.
        assert_eq!(auto_switch_target(&rules, &profiles, "work", 0, "VALORANT.exe", "", true), Some(("valorant".into(), 0)));
    }
}
