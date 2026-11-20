use super::deps::*;
use super::appearance::*;
use super::routing::LiveAction;

/// Windows media/COM calls may block. Never hold app state while sampling them.
pub(crate) fn sample_face(shared: &SharedState) -> bool {
    let (needs_apps, needs_foreground, face_needs) = {
        let guard = shared.lock_safe();
        let uses = |page: &ProfilePage| {
            page.keys.iter().any(|k| k.action_id.starts_with("audio.app_"))
                || page.touch_zones.iter().any(|z| z.tap_action_id.starts_with("audio.app_"))
                || page.encoders.iter().any(|e| [&e.cw_action, &e.ccw_action, &e.push_action].iter().any(|a| a.starts_with("audio.app_")))
        };
        let uses_foreground = |page: &ProfilePage| {
            let is_target = |action: &str, param: &str| action.starts_with("audio.app_") && param.trim().eq_ignore_ascii_case(actions::app_audio::FOREGROUND_TARGET);
            page.keys.iter().any(|k| is_target(&k.action_id, &k.action_param))
                || page.touch_zones.iter().any(|z| is_target(&z.tap_action_id, &z.tap_action_param))
                || page.encoders.iter().any(|e| is_target(&e.cw_action, &e.cw_param) || is_target(&e.ccw_action, &e.ccw_param) || is_target(&e.push_action, &e.push_param))
        };
        let applied = guard.applied_layout.as_ref().map(|a| &a.page);
        // Once a page is on the deck, only that page pays for media, volume, and stats.
        // Before the first apply, the editor still samples whatever any page displays.
        let face_needs = match applied {
            Some(page) => page_face_needs(page),
            None => guard.profiles.iter().flat_map(|profile| &profile.pages).fold(FaceNeeds::default(), |acc, page| acc.or(page_face_needs(page))),
        };
        let mut pages = guard.profiles.iter().flat_map(|p| &p.pages);
        (pages.clone().any(uses) || applied.is_some_and(uses),
         pages.any(uses_foreground) || applied.is_some_and(uses_foreground),
         face_needs)
    };
    sample_face_with(shared, |previous| read_face(previous, needs_apps, needs_foreground, face_needs))
}

pub(crate) fn sample_face_with(
    shared: &SharedState,
    sample: impl FnOnce(&DeckFaceState) -> DeckFaceState,
) -> bool {
    let (previous, refresh_requested) = {
        let mut guard = shared.lock_safe();
        let refresh_requested = guard.sample_after_ms > 0;
        guard.sample_after_ms = 0;
        (guard.face.clone(), refresh_requested)
    };
    let mut next = sample(&previous);
    let mut guard = shared.lock_safe();
    stamp_latch(&guard, &mut next);
    let changed = next != guard.face;
    if changed {
        guard.face = next;
    }
    // Requests arriving during the OS query remain scheduled for the next pass.
    changed || refresh_requested
}

#[derive(Clone, Copy, Default)]
pub(crate) struct FaceNeeds {
    pub(crate) media: bool,
    pub(crate) audio: bool,
    pub(crate) stats: bool,
}

impl FaceNeeds {
    fn or(self, other: Self) -> Self {
        Self { media: self.media || other.media, audio: self.audio || other.audio, stats: self.stats || other.stats }
    }
}

fn note_face_action(needs: &mut FaceNeeds, action: &str) {
    match action {
        "media.play_pause" | "media.now_playing" | "media.next" | "media.prev" | "media.stop" => needs.media = true,
        "audio.mute_mic" | "audio.volume_mute" | "audio.volume_up" | "audio.volume_down" => needs.audio = true,
        "sys.cpu" | "sys.ram" | "sys.clock" => needs.stats = true,
        _ => {}
    }
}

pub(crate) fn page_face_needs(page: &ProfilePage) -> FaceNeeds {
    let mut needs = FaceNeeds::default();
    for key in &page.keys {
        note_face_action(&mut needs, &key.action_id);
    }
    for zone in &page.touch_zones {
        note_face_action(&mut needs, &zone.tap_action_id);
    }
    for encoder in &page.encoders {
        note_face_action(&mut needs, &encoder.cw_action);
        note_face_action(&mut needs, &encoder.ccw_action);
        note_face_action(&mut needs, &encoder.push_action);
    }
    needs
}

fn read_face(previous: &DeckFaceState, needs_apps: bool, needs_foreground: bool, face_needs: FaceNeeds) -> DeckFaceState {
    let _timing = performance::timer(performance::Stage::Media);
    let (playing, title) = if face_needs.media {
        read_media_face()
    } else {
        (previous.media_playing, previous.media_title.clone())
    };
    let (mic_muted, speaker_muted, volume_percent) = if face_needs.audio {
        let audio = read_audio_face();
        (audio.mic_muted, audio.speaker_muted, audio.volume_percent)
    } else {
        (previous.mic_muted, previous.speaker_muted, previous.volume_percent)
    };
    let (cpu_percent, ram_percent, clock) = if face_needs.stats {
        let stats = crate::actions::read_quick_stats();
        (stats.cpu_percent, stats.ram_percent, stats.clock)
    } else {
        (previous.cpu_percent, previous.ram_percent, previous.clock.clone())
    };
    let media_art = if !face_needs.media {
        previous.media_art.clone()
    } else if previous.media_title == title && !previous.media_art.is_empty() {
        previous.media_art.clone()
    } else if playing.is_some() {
        crate::actions::read_media_art()
    } else {
        Vec::new()
    };
    DeckFaceState {
        media_playing: playing,
        media_title: title,
        mic_muted,
        speaker_muted,
        volume_percent,
        app_audio: if needs_apps { actions::app_audio::list().ok().map(|apps| apps.into_iter().map(|app| (app.exe.clone(), app)).collect()) } else { None },
        foreground_audio_exe: if needs_foreground { foreground_window().map(|window| window.exe_name.to_lowercase()).filter(|exe| exe != "deckforge.exe") } else { None },
        game_mode: Some(game_mode::is_active()),
        game_mode_busy: game_mode::is_busy(),
        cpu_percent,
        ram_percent,
        clock,
        media_art,
        plugin_keys: crate::plugins::current_faces(),
        latched_keys: Vec::new(),
        latched_profile_id: String::new(),
        latched_page_id: String::new(),
    }
}

fn latch_view(latched: &std::collections::HashSet<String>, layout: Option<&AppliedLayout>) -> (Vec<u8>, String, String) {
    let Some(layout) = layout else { return (Vec::new(), String::new(), String::new()) };
    let prefix = format!("{}/{}/", layout.profile_id, layout.page.id);
    let mut keys: Vec<u8> = latched.iter().filter_map(|id| id.strip_prefix(&prefix)?.parse().ok()).collect();
    keys.sort_unstable();
    (keys, layout.profile_id.clone(), layout.page.id.clone())
}

fn stamp_latch(guard: &ManagedAppState, face: &mut DeckFaceState) {
    let (keys, profile_id, page_id) = latch_view(&guard.latched, guard.applied_layout.as_ref());
    face.latched_keys = keys;
    face.latched_profile_id = profile_id;
    face.latched_page_id = page_id;
}

/// Flips a latched key and asks the deck to repaint. Keys without Stay lit are left alone.
pub(crate) fn note_latch(shared: &SharedState, action: &LiveAction) {
    let Some(index) = action.key_index else { return };
    let mut guard = shared.lock_safe();
    let Some(layout) = guard.applied_layout.clone() else { return };
    let Some(key) = layout.page.keys.iter().find(|key| key.index == index) else { return };
    if !key.latch { return };
    let id = format!("{}/{}/{}", layout.profile_id, layout.page.id, index);
    if !guard.latched.remove(&id) {
        guard.latched.insert(id);
    }
    let (keys, profile_id, page_id) = latch_view(&guard.latched, guard.applied_layout.as_ref());
    guard.face.latched_keys = keys;
    guard.face.latched_profile_id = profile_id;
    guard.face.latched_page_id = page_id;
    guard.sample_after_ms = now_ms();
}

pub(crate) fn push_face_deltas(shared: &SharedState, app: &tauri::AppHandle) {
    let _display = DISPLAY_WRITE.lock_safe();
    let (face, reactive_target) = {
        let guard = shared.lock_safe();
        let target = if guard.layout_ready {
            guard.applied_layout.as_ref().and_then(|applied| {
                if !applied.page.encoders.iter().any(|e| e.led_reactive) {
                    return None;
                }
                let dev = guard.devices.iter().find(|d| d.id == guard.playback_device_id && d.connected)?.clone();
                let colors = effective_page_knob_colors(&applied.page, &guard.face);
                if guard.last_reactive_knob_leds.as_ref() == Some(&colors) {
                    return None;
                }
                Some((dev, colors))
            })
        } else {
            None
        };
        (guard.face.clone(), target)
    };
    let _ = app.emit("deck-face", &face);
    if let Some((dev, colors)) = reactive_target {
        let mut diags = Vec::new();
        if send_hardware_knob_leds(&dev, &colors, &mut diags).is_ok() {
            let mut guard = shared.lock_safe();
            guard.custom_knob_leds_active = true;
            guard.last_reactive_knob_leds = Some(colors);
        }
    }
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

pub(crate) struct RenderedFaceFrame {
    pub(crate) slot: String,
    pub(crate) jpeg: Vec<u8>,
    /// Title and icon bytes mixed into the frame hash. Kept off the JPEG allocation.
    pub(crate) hash_extra: Vec<u8>,
    pub(crate) image_slot: u8,
}

/// Caller holds DISPLAY_WRITE until transmission finishes, keeping the applied
/// page and frame-cache batch stable while expensive rendering releases state.
pub(crate) fn face_delta_bursts(shared: &SharedState, face: &DeckFaceState) -> Option<(DeviceDescriptor, Vec<HidBurst>)> {
    face_delta_bursts_with(shared, face, render_face_frames)
}

pub(crate) fn face_delta_bursts_with(
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
            guard.frame_cache.should_transmit_parts(&dev.id, &frame.slot, &[&frame.jpeg, &frame.hash_extra]).is_some()
        }).collect::<Vec<_>>()
    };
    let bursts = pending.into_iter().map(|frame| drivers::hal::key_image_burst(&dev, frame.image_slot, &frame.jpeg)).collect();
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
        let resting = title == key.title && icon == key.icon_name && level.is_none() && art.is_none();
        if resting && !key.latch {
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
        let mut hash_extra = Vec::with_capacity(title.len() + icon.len());
        hash_extra.extend_from_slice(title.as_bytes());
        hash_extra.extend_from_slice(icon.as_bytes());
        let slot = format!("key-{}", key.index);
        bursts.push(RenderedFaceFrame {
            slot, hash_extra, jpeg,
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
                slot, hash_extra: Vec::new(), jpeg,
                image_slot: touch_zone_image_slot(dev, zone.index),
            });
        }
    }
    bursts
}
