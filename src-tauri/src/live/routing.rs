use super::deps::*;

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

pub(crate) fn key_gesture_bindings(guard: &ManagedAppState, index: u8) -> GestureBindings {
    let Some(page) = guard.applied_layout.as_ref().map(|layout| &layout.page) else {
        return GestureBindings { has_double: false, has_hold: false };
    };
    let Some(key) = page.keys.iter().find(|key| key.index == index) else {
        return GestureBindings { has_double: false, has_hold: false };
    };
    GestureBindings {
        has_double: key.double_press.as_ref().is_some_and(KeyGesture::is_set),
        has_hold: key.hold.as_ref().is_some_and(KeyGesture::is_set),
    }
}

pub(crate) fn action_for_key_gesture(guard: &ManagedAppState, device_id: &str, index: u8, kind: GestureKind) -> Option<LiveAction> {
    if !guard.layout_ready {
        return None;
    }
    let page = &guard.applied_layout.as_ref()?.page;
    let key = page.keys.iter().find(|key| key.index == index)?;
    let (action_id, label, param, note) = match kind {
        GestureKind::Tap => {
            if key.action_id.is_empty() || key.action_id == "none" {
                return None;
            }
            (key.action_id.clone(), key.action_label.clone(), key.action_param.clone(), format!("Key #{} ({})", index + 1, key.title))
        }
        GestureKind::Double => {
            let gesture = key.double_press.as_ref().filter(|gesture| gesture.is_set())?;
            (gesture.action_id.clone(), gesture.action_label.clone(), gesture.action_param.clone(), format!("Key #{} ({}) double-press", index + 1, key.title))
        }
        GestureKind::Hold => {
            let gesture = key.hold.as_ref().filter(|gesture| gesture.is_set())?;
            (gesture.action_id.clone(), gesture.action_label.clone(), gesture.action_param.clone(), format!("Key #{} ({}) hold", index + 1, key.title))
        }
    };
    Some(LiveAction {
        action_id,
        label,
        param,
        note,
        device_id: device_id.to_string(),
        key_index: Some(index),
    })
}

pub(crate) fn action_for_event(guard: &ManagedAppState, evt: &HardwareInputEvent) -> Option<LiveAction> {
    if !guard.layout_ready {
        return None;
    }
    let page = &guard.applied_layout.as_ref()?.page;
    match evt.kind {
        InputControlKind::LcdKey => action_for_key_gesture(guard, &evt.device_id, evt.control_index, GestureKind::Tap),
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
            | "audio.play_sound"
            | "audio.stop_sound"
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
