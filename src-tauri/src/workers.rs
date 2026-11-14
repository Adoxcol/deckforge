use super::*;

pub(crate) fn spawn_desktop_worker(app: tauri::AppHandle, shared: SharedState) {
    std::thread::spawn(move || {
        let mut stable_exe = String::new();
        let mut stable_title = String::new();
        let mut stable_since = 0u64;
        let mut last_sample = 0u64;
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let now = now_ms();
            let should_sample = {
                let guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                (guard.sample_after_ms > 0 && now >= guard.sample_after_ms) || now.saturating_sub(last_sample) > 2000
            };
            if should_sample {
                last_sample = now;
                let changed = sample_face(&shared);
                // Only tell the window when something changed: each update redraws the editor.
                if changed {
                    push_face_deltas(&shared, &app);
                }
            }
            if let Some(window) = foreground_window() {
                let ignored = window.exe_name.eq_ignore_ascii_case("deckforge.exe");
                if ignored {
                    continue;
                }
                if window.exe_name != stable_exe || window.title != stable_title {
                    stable_exe = window.exe_name.clone();
                    stable_title = window.title.clone();
                    stable_since = now;
                    continue;
                }
                if now.saturating_sub(stable_since) < 800 {
                    continue;
                }
                // Only ask about games when some rule cares; detection reads window geometry.
                let wants_game = {
                    let guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                    guard.auto_switch_rules.iter().any(|r| r.enabled && r.process_name.trim().eq_ignore_ascii_case(crate::live::ANY_GAME))
                };
                let is_game = wants_game && crate::game_mode::foreground_is_game();
                let switch_to = {
                    let guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                    if now < guard.auto_switch_hold_until_ms {
                        None
                    } else {
                        auto_switch_target(
                            &guard.auto_switch_rules,
                            &guard.profiles,
                            &guard.active_profile_id,
                            guard.active_page_index,
                            &stable_exe,
                            &stable_title,
                            is_game,
                        )
                    }
                };
                if let Some((profile_id, page_index)) = switch_to {
                    let _display = DISPLAY_WRITE.lock_safe();
                    let event = {
                        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                        let name = guard
                            .profiles
                            .iter()
                            .find(|profile| profile.id == profile_id)
                            .map(|profile| match profile.pages.get(page_index) {
                                Some(page) if page_index > 0 => format!("{} / {}", profile.name, page.name),
                                _ => profile.name.clone(),
                            })
                            .unwrap_or_else(|| profile_id.clone());
                        guard.active_profile_id = profile_id.clone();
                        guard.active_page_index = page_index;
                        DeckContextEvent {
                            profile_id,
                            page_index,
                            message: format!("{stable_exe} is in front. Switched to {name}"),
                        }
                    };
                    if push_active_layout_locked(&shared, false).is_ok() {
                        let _ = app.emit("deck-context", &event);
                    }
                    stable_since = now_ms();
                }
            }
        }
    });
}

/// Spawns a dedicated background Rust thread that listens for `ACK\0\0OK` input reports
/// from the physical Ajazz AKP05E Pro (`VID_0300&PID_3013`, `MI_00`), emits `"hardware-input"`
/// events to the UI, and dispatches bound Win32 actions when buttons/knobs are pressed.
enum InputWork {
    FullLayout(u64),
    Press {
        action: Option<LiveAction>,
        ts: u64,
        raw_hex: String,
    },
}

fn run_input_work(app_handle: &tauri::AppHandle, shared: &SharedState, work: InputWork) {
    let (action, ts, raw_hex) = match work {
        InputWork::FullLayout(generation) => {
            // Several resend/reconnect requests may arrive while an action is
            // running. Only the newest generation should reset the panel.
            if generation == deck_generation() { push_connected_layout(shared); }
            return;
        }
        InputWork::Press { action, ts, raw_hex } => (action, ts, raw_hex),
    };
    // Presses suppressed during a layout transition must not queue extra wake
    // writes behind that transfer and delay the next valid action.
    let Some(action) = action else { return; };
    if !drivers::hal::deck_control_available() { return; }
    let dev = shared
        .lock_safe()
        .devices
        .iter()
        .find(|d| d.connected && d.protocol_family.starts_with("CRT"))
        .cloned();
    if let Some(dev) = dev {
        let _ = wake_panel(&dev);
    }
    let result = dispatch_live_action(app_handle, shared, &action);
    let mut guard = shared.lock_safe();
    guard.diagnostics.push(DiagnosticEntry {
        id: format!("diag-hw-{}", ts),
        timestamp_ms: ts,
        level: if result.success { "success" } else { "error" }.into(),
        device_id: action.device_id,
        stage: "HW_KEY_PRESS".to_string(),
        message: format!("{} → {}", action.note, result.output_message),
        packet_hex: Some(raw_hex),
    });
    if guard.diagnostics.len() > 80 {
        let extra = guard.diagnostics.len() - 80;
        guard.diagnostics.drain(0..extra);
    }
}

pub(crate) fn spawn_hardware_input_worker(app_handle: tauri::AppHandle, shared: SharedState) {
    // Actions, panel wake-ups and full layout pushes can take seconds (macros sleep, a
    // layout push retries). Running them here stopped the deck's buttons from being read
    // meanwhile, so they go to a second thread that handles them in order.
    let (work_tx, work_rx) = std::sync::mpsc::channel::<InputWork>();
    {
        let app_handle = app_handle.clone();
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            for work in work_rx {
                run_input_work(&app_handle, &shared, work);
            }
        });
    }
    std::thread::spawn(move || {
        let mut buf = [0u8; 2048];
        let mut held: Vec<(u8, u8, bool)> = Vec::new();
        let mut last_tap_ms: Vec<(u8, u8, u64)> = Vec::new();
        let mut strip_gesture = StripGesture::default();
        let mut seen_generation = 0u64;
        loop {
            let generation = deck_generation();
            if generation != seen_generation && generation > 0 {
                // The deck was opened (first start, or after a real unplug). Only
                // then do the pictures go out in full.
                seen_generation = generation;
                held.clear();
                last_tap_ms.clear();
                strip_gesture = StripGesture::default();
                let _ = work_tx.send(InputWork::FullLayout(generation));
                continue;
            }
            let Some(raw) = deck_next_report(60) else {
                continue;
            };
            let n = raw.len().min(buf.len());
            buf[..n].copy_from_slice(&raw[..n]);
            {
                {
                        {
                            let ts = now_ms();
                            let Some(mut evt) = parse_input_report("ajazz-akp05e-pro", &buf[..n], ts) else {
                                continue;
                            };
                            if evt.kind == InputControlKind::TouchStripPoint {
                                let Some(resolved) = resolve_strip_point(&mut strip_gesture, &evt) else {
                                    continue;
                                };
                                evt = resolved;
                            }
                            if matches!(evt.kind, InputControlKind::Unknown) {
                                continue;
                            }
                            let is_turn = matches!(
                                evt.kind,
                                InputControlKind::RotaryKnobTurnCw | InputControlKind::RotaryKnobTurnCcw
                            );
                            let one_shot = matches!(
                                evt.kind,
                                InputControlKind::TouchStripTap
                                    | InputControlKind::TouchStripSwipe
                                    | InputControlKind::RotaryKnobPush
                            );
                            let slot = (kind_code(&evt.kind), evt.control_index);
                            if one_shot {
                                let prev = last_tap_ms
                                    .iter()
                                    .find(|(k, i, _)| *k == slot.0 && *i == slot.1)
                                    .map(|(_, _, t)| *t)
                                    .unwrap_or(0);
                                if let Some(entry) = last_tap_ms.iter_mut().find(|(k, i, _)| *k == slot.0 && *i == slot.1) {
                                    entry.2 = ts;
                                } else {
                                    last_tap_ms.push((slot.0, slot.1, ts));
                                }
                                // A press and its release arrive as two reports. A swipe can repeat.
                                let gap = if matches!(evt.kind, InputControlKind::TouchStripSwipe) {
                                    450
                                } else {
                                    280
                                };
                                if ts.saturating_sub(prev) < gap {
                                    continue;
                                }
                            } else if !is_turn {
                                let was = held
                                    .iter()
                                    .find(|(k, i, _)| *k == slot.0 && *i == slot.1)
                                    .map(|(_, _, down)| *down)
                                    .unwrap_or(false);
                                if let Some(entry) = held.iter_mut().find(|(k, i, _)| *k == slot.0 && *i == slot.1) {
                                    entry.2 = evt.pressed;
                                } else {
                                    held.push((slot.0, slot.1, evt.pressed));
                                }
                                if evt.pressed == was {
                                    continue;
                                }
                            }
                            let _ = app_handle.emit("hardware-input", &evt);
                            let pending = if evt.pressed || is_turn {
                                Some(shared.lock_safe()).and_then(|mut guard| {
                                    guard.recent_input_events.push(evt.clone());
                                    if guard.recent_input_events.len() > 25 {
                                        guard.recent_input_events.remove(0);
                                    }
                                    action_for_event(&guard, &evt)
                                })
                            } else {
                                None
                            };
                            if evt.pressed || is_turn {
                                let _ = work_tx.send(InputWork::Press {
                                    action: pending,
                                    ts,
                                    raw_hex: evt.raw_hex.clone(),
                                });
                            }
                        }
                }
            }
        }
    });
}

/// Cycles GIF frames onto key LCDs and the touch strip. At most three screens are
/// written per tick so the USB bus stays responsive.
pub(crate) fn spawn_animation_worker(shared: SharedState) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(40));
        let _ = animation_tick(&shared, write_animation_frames);
    });
}

pub(crate) fn animation_tick(
    shared: &SharedState,
    write: impl FnOnce(&DeviceDescriptor, &[HidBurst]) -> Result<(), String>,
) -> Result<(), String> {
    animation_tick_with(shared, build_animation_bursts, write)
}

pub(crate) fn animation_tick_with(
    shared: &SharedState,
    build: impl FnOnce(&DeviceDescriptor, Vec<(PlaybackTarget, Vec<u8>)>) -> Vec<HidBurst>,
    write: impl FnOnce(&DeviceDescriptor, &[HidBurst]) -> Result<(), String>,
) -> Result<(), String> {
    // A full layout owns the display until its bindings have been committed.
    let Ok(_display) = DISPLAY_WRITE.try_lock() else { return Ok(()); };
    let due = {
        let mut guard = shared.lock_safe();
        if !guard.layout_ready { return Ok(()); }
        let now = now_ms();
        let dev = guard
            .devices
            .iter()
            .find(|d| d.id == guard.playback_device_id && d.connected)
            .cloned();
        let Some(dev) = dev else { return Ok(()); };
        let previous_slots: Vec<_> = guard.playback_slots.iter()
            .map(|slot| (slot.frame_index, slot.next_due_ms)).collect();
        let mut pending = Vec::new();
        // Serve the longest-waiting slots first. Always starting at index zero
        // can starve later animations when a transfer exceeds the frame delay.
        let mut due_slots: Vec<_> = guard.playback_slots.iter().enumerate()
            .filter(|(_, slot)| now >= slot.next_due_ms && !slot.jpegs.is_empty())
            .map(|(index, slot)| (slot.next_due_ms, index)).collect();
        due_slots.sort_unstable();
        for (_, index) in due_slots.into_iter().take(3) {
            let slot = &mut guard.playback_slots[index];
            let idx = slot.frame_index.min(slot.jpegs.len() - 1);
            let jpeg = slot.jpegs[idx].clone();
            let delay = slot.delays_ms.get(idx).copied().unwrap_or(100).max(80);
            let slot_id = slot.slot_id.clone();
            let target = slot.target.clone();
            if slot.frame_index + 1 >= slot.jpegs.len() {
                if slot.loop_playback {
                    slot.frame_index = 0;
                } else {
                    slot.frame_index = slot.jpegs.len() - 1;
                    slot.next_due_ms = u64::MAX;
                }
            } else {
                slot.frame_index += 1;
            }
            if slot.next_due_ms != u64::MAX {
                slot.next_due_ms = now.saturating_add(delay);
            }
            pending.push((slot_id, target, jpeg));
        }
        let mut frames = Vec::new();
        for (slot_id, target, jpeg) in pending {
            if guard.frame_cache.should_transmit(&dev.id, &slot_id, &jpeg).is_some() {
                frames.push((target, jpeg));
            }
        }
        if frames.is_empty() {
            None
        } else {
            Some((dev, frames, previous_slots))
        }
    };
    if let Some((dev, frames, previous_slots)) = due {
        let bursts = build(&dev, frames);
        {
            let guard = shared.lock_safe();
            if !guard.layout_ready || !guard.devices.iter().any(|d| d.id == dev.id && d.connected) {
                return Ok(());
            }
        }
        if let Err(err) = transmit_cached_frames(shared, &dev, &bursts, write) {
            let mut guard = shared.lock_safe();
            let retry_at = now_ms().saturating_add(250);
            for (slot, (frame_index, next_due_ms)) in guard.playback_slots.iter_mut().zip(previous_slots) {
                slot.frame_index = frame_index;
                slot.next_due_ms = next_due_ms.max(retry_at);
            }
            return Err(err);
        }
    }
    Ok(())
}

pub(crate) fn build_animation_bursts(dev: &DeviceDescriptor, frames: Vec<(PlaybackTarget, Vec<u8>)>) -> Vec<HidBurst> {
    let mut bursts = Vec::new();
    for (target, jpeg) in frames {
        let reports = match target {
            PlaybackTarget::Key(index) => {
                build_key_image_packets(key_image_slot(&dev, index), &jpeg)
            }
            PlaybackTarget::TouchZone(index) => {
                build_key_image_packets(touch_zone_image_slot(&dev, index), &jpeg)
            }
            PlaybackTarget::Strip { width, height } => {
                build_touch_strip_packets(&jpeg, 0, 0, width, height)
            }
        };
        bursts.push(HidBurst {
            reports: reports.into_iter().map(|r| r.to_vec()).collect(),
        });
    }
    bursts
}
