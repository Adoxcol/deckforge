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

pub(crate) fn push_connected_layout(shared: &SharedState, usb: &Arc<Mutex<()>>) {
    if !ensure_deck_in_state(shared) {
        deck_note("no deck in the device list, so the layout was not sent");
        return;
    }
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
    //
    // A failed send is tried again. When the deck dropped off USB in the middle of it, the
    // IO thread opens it again and the input worker sends the layout from scratch, so
    // this loop gives up as soon as the deck has been reopened.
    let generation = deck_generation();
    for attempt in 1..=3u32 {
        match push_active_layout(shared, usb, true) {
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

pub(crate) fn push_active_layout(shared: &SharedState, usb: &Arc<Mutex<()>>, handshake: bool) -> Result<(), String> {
    let prepared = {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        prepare_layout_bursts(&mut guard, handshake)
    };
    let Some((dev, bursts)) = prepared else {
        return Ok(());
    };
    if bursts.is_empty() {
        return Ok(());
    }
    let _bus = usb.lock().unwrap_or_else(|e| e.into_inner());
    if handshake {
        write_hid_bursts(&dev, &bursts)
    } else {
        // A page change only replaces the pictures. HAN would fade the backlight off.
        write_animation_frames(&dev, &bursts)
    }
}

pub(crate) fn prepare_layout_bursts(guard: &mut ManagedAppState, invalidate: bool) -> Option<(DeviceDescriptor, Vec<HidBurst>)> {
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

pub(crate) fn dispatch_live_action(
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
pub(crate) fn nudge_face_sample(shared: &SharedState) {
    if let Ok(mut guard) = shared.lock() {
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
    let _ = push_active_layout(shared, usb, false);
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

pub(crate) fn push_face_deltas(shared: &SharedState, usb: &Arc<Mutex<()>>, app: &tauri::AppHandle) {
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

pub(crate) fn face_delta_bursts(guard: &mut ManagedAppState, face: &DeckFaceState) -> Option<(DeviceDescriptor, Vec<HidBurst>)> {
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
    use super::{binding_action, known_action};

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
