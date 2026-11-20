use super::deps::*;

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
    let foreground = param.trim().eq_ignore_ascii_case(actions::app_audio::FOREGROUND_TARGET);
    let exe = if foreground {
        match face.foreground_audio_exe.as_deref() { Some(exe) => exe, None => return "Focus app".into() }
    } else { param.trim() };
    match &face.app_audio {
        None => "Unavailable".into(),
        Some(apps) => {
            let state = apps.get(&exe.to_lowercase()).map(|app| app.label()).unwrap_or_else(|| "No session".into());
            if foreground { format!("{} {state}", exe.trim_end_matches(".exe")) } else { state }
        },
    }
}

fn audio_face_target<'a>(param: &'a str, face: &'a DeckFaceState) -> Option<&'a str> {
    if param.trim().eq_ignore_ascii_case(actions::app_audio::FOREGROUND_TARGET) { face.foreground_audio_exe.as_deref() }
    else { Some(param.trim()) }
}

pub(crate) fn key_level_for(key: &KeyConfig, face: &DeckFaceState) -> Option<u8> {
    if key.action_id.starts_with("audio.app_") {
        let app = face.app_audio.as_ref()?.get(&audio_face_target(&key.action_param, face)?.to_lowercase())?;
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
    if key.latch && face.latched_keys.contains(&key.index) {
        return (format!("ON {}", key.title), "Check".to_string());
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KnobLedSyncAction {
    None,
    Set(Vec<String>),
    Reset,
}

pub(crate) fn reactive_encoder_color(enc: &db::EncoderConfig, face: &DeckFaceState) -> Option<&'static str> {
    if !enc.led_reactive {
        return None;
    }
    let slots = [
        (&enc.push_action, &enc.push_param),
        (&enc.cw_action, &enc.cw_param),
        (&enc.ccw_action, &enc.ccw_param),
    ];
    for (action, param) in slots {
        match action.as_str() {
            "audio.mute_mic" if face.mic_muted == Some(true) => return Some("#F16B79"),
            "audio.volume_mute" | "audio.volume_up" | "audio.volume_down"
                if face.speaker_muted == Some(true) =>
            {
                return Some("#F16B79")
            }
            "audio.app_mute" | "audio.app_volume_up" | "audio.app_volume_down" => {
                let target = if param.trim().eq_ignore_ascii_case(actions::app_audio::FOREGROUND_TARGET) {
                    face.foreground_audio_exe.as_deref().unwrap_or("")
                } else {
                    param.trim()
                };
                if !target.is_empty() {
                    if let Some(apps) = &face.app_audio {
                        let lower = target.to_lowercase();
                        if apps.iter().any(|(exe, st)| exe.eq_ignore_ascii_case(&lower) && st.muted) {
                            return Some("#F16B79");
                        }
                    }
                }
            }
            "sys.game_mode" if face.game_mode == Some(true) => return Some("#35C98B"),
            "media.play_pause" | "media.next" | "media.prev"
                if face.media_playing == Some(true) =>
            {
                return Some("#35C98B")
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn effective_page_knob_colors(page: &ProfilePage, face: &DeckFaceState) -> Vec<String> {
    (0..4)
        .map(|i| {
            page.encoders
                .iter()
                .find(|e| e.index as usize == i)
                .map(|e| {
                    reactive_encoder_color(e, face)
                        .map(str::to_string)
                        .unwrap_or_else(|| e.led_color.clone())
                })
                .unwrap_or_default()
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn plan_page_knob_leds(page: &ProfilePage, custom_active: bool) -> KnobLedSyncAction {
    plan_page_knob_leds_with_face(page, &DeckFaceState::default(), custom_active)
}

pub(crate) fn plan_page_knob_leds_with_face(
    page: &ProfilePage,
    face: &DeckFaceState,
    custom_active: bool,
) -> KnobLedSyncAction {
    let has_override = page.encoders.iter().any(|e| {
        !e.led_color.trim().is_empty() || reactive_encoder_color(e, face).is_some()
    });
    if has_override {
        KnobLedSyncAction::Set(effective_page_knob_colors(page, face))
    } else if custom_active {
        KnobLedSyncAction::Reset
    } else {
        KnobLedSyncAction::None
    }
}
