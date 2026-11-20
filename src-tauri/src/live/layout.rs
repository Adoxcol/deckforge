use super::deps::*;
use super::appearance::*;

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
    let (custom_active, face) = {
        let guard = shared.lock_safe();
        (guard.custom_knob_leds_active, guard.face.clone())
    };
    let action = plan_page_knob_leds_with_face(page, &face, custom_active);
    let mut diags = Vec::new();
    match action {
        KnobLedSyncAction::None => {
            let mut guard = shared.lock_safe();
            guard.live_knob_led_effect = page.knob_led_effect.clone();
            guard.live_knob_led_base = drivers::hal::parse_knob_rgb_list(&effective_page_knob_colors(page, &face));
            return;
        }
        KnobLedSyncAction::Set(colors) => {
            let rgb = drivers::hal::parse_knob_rgb_list(&colors);
            if send_hardware_knob_leds(dev, &colors, &mut diags).is_ok() {
                let mut guard = shared.lock_safe();
                guard.custom_knob_leds_active = true;
                guard.last_reactive_knob_leds = Some(colors);
                guard.live_knob_led_effect = page.knob_led_effect.clone();
                guard.live_knob_led_base = rgb;
            }
        }
        KnobLedSyncAction::Reset => {
            if reset_hardware_knob_leds(dev, &mut diags).is_ok() {
                let mut guard = shared.lock_safe();
                guard.custom_knob_leds_active = false;
                guard.last_reactive_knob_leds = None;
                guard.live_knob_led_effect = page.knob_led_effect.clone();
                guard.live_knob_led_base = [
                    drivers::hal::default_knob_rgb(0),
                    drivers::hal::default_knob_rgb(1),
                    drivers::hal::default_knob_rgb(2),
                    drivers::hal::default_knob_rgb(3),
                ];
            }
        }
    }
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

pub(crate) fn prepare_reconnect(guard: &mut ManagedAppState) -> Result<PreparedLayout, String> {
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
pub(crate) fn render_layout_frames(prepared: &mut PreparedLayout) -> (drivers::hal::PublishOutcome, Vec<DiagnosticEntry>) {
    let _timing = performance::timer(performance::Stage::FullRender);
    let (jobs, zones, strip) = plan_screens(&prepared.dev, &prepared.applied.page, &prepared.face);
    let mut diags = Vec::new();
    let outcome = publish_screens(
        &prepared.dev, &jobs, &zones, strip.as_ref(), &prepared.media,
        &mut prepared.frame_cache, &mut diags,
    );
    (outcome, diags)
}

pub(crate) fn render_layout_with(
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
