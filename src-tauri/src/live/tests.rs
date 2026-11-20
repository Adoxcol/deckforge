mod live_tests {
    use super::*;
    use super::deps::*;

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
            custom_knob_leds_active: false,
            last_reactive_knob_leds: None,
            live_knob_led_effect: String::new(),
            live_knob_led_base: [
                drivers::hal::default_knob_rgb(0),
                drivers::hal::default_knob_rgb(1),
                drivers::hal::default_knob_rgb(2),
                drivers::hal::default_knob_rgb(3),
            ],
            latched: std::collections::HashSet::new(),
        }))
    }

    #[test]
    fn reactive_knob_led_turns_red_when_muted() {
        let mut page = db::default_profiles().remove(0).pages.remove(0);
        page.encoders[0].led_reactive = true;
        page.encoders[0].led_color = "#35C98B".into();
        let mut face = DeckFaceState::default();
        assert_eq!(reactive_encoder_color(&page.encoders[0], &face), None);
        face.speaker_muted = Some(true);
        assert_eq!(reactive_encoder_color(&page.encoders[0], &face), Some("#F16B79"));
        assert_eq!(effective_page_knob_colors(&page, &face)[0], "#F16B79");
    }

    #[test]
    fn page_without_custom_leds_resets_after_custom_led_page() {
        let mut page = db::default_profiles().remove(0).pages.remove(0);
        assert_eq!(plan_page_knob_leds(&page, false), KnobLedSyncAction::None);
        assert_eq!(plan_page_knob_leds(&page, true), KnobLedSyncAction::Reset);
        page.encoders[0].led_color = "#FF0000".into();
        assert_eq!(
            plan_page_knob_leds(&page, false),
            KnobLedSyncAction::Set(vec!["#FF0000".into(), "".into(), "".into(), "".into()])
        );
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
                slot: "key-0".into(), jpeg: vec![1], hash_extra: Vec::new(), image_slot: 1,
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
            corner_radius: 8, media: None, double_press: None, hold: None, latch: false,
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
        let mut latched = key("custom.hotkey");
        latched.latch = true;
        latched.title = "Camera".into();
        assert_eq!(styled_key(&latched, &DeckFaceState::default()).0, "Camera");
        let on = DeckFaceState { latched_keys: vec![0], ..Default::default() };
        assert_eq!(styled_key(&latched, &on), ("ON Camera".to_string(), "Check".to_string()));
    }

    #[test]
    fn the_applied_page_decides_which_face_reads_run() {
        let key = |action: &str| KeyConfig {
            index: 0, title: "Key".into(), subtitle: String::new(), icon_name: "Cpu".into(),
            action_id: action.into(), action_label: String::new(), action_category: String::new(),
            action_param: String::new(), accent_color: "#fff".into(), bg_color: "#000".into(),
            show_title: true, title_size: "Medium".into(), background_style: "Gradient".into(),
            corner_radius: 8, media: None, double_press: None, hold: None, latch: false,
        };
        let page = ProfilePage {
            id: "p".into(), name: "Page".into(), keys: vec![key("sys.cpu")],
            touch_zones: vec![], encoders: vec![], strip_media: None, knob_led_effect: String::new(),
        };
        let needs = page_face_needs(&page);
        assert!(needs.stats);
        assert!(!needs.media && !needs.audio);
        let with_knob = ProfilePage {
            encoders: vec![db::EncoderConfig {
                index: 0, label: String::new(), value_text: String::new(),
                cw_action: "audio.volume_up".into(), ccw_action: String::new(), push_action: "media.play_pause".into(),
                cw_param: String::new(), ccw_param: String::new(), push_param: String::new(),
                led_color: String::new(), led_reactive: false,
            }],
            ..page
        };
        let needs = page_face_needs(&with_knob);
        assert!(needs.stats && needs.audio && needs.media);
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
        ProfilePage { id: id.into(), name: id.into(), keys: vec![], touch_zones: vec![], encoders: vec![], strip_media: None, knob_led_effect: String::new() }
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
    fn deleted_page_rule_does_not_switch_profiles() {
        let rules = [rule("resolve.exe", "edit", "gone", 1)];
        assert_eq!(auto_switch_target(&rules, &profiles(), "stream", 1, "resolve.exe", "", false), None);
        assert_eq!(auto_switch_target(&rules, &profiles(), "edit", 0, "resolve.exe", "", false), None);
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
    fn local_page_rule_and_global_profile_rule_have_separate_scopes() {
        let rules = [rule("obs64.exe", "edit", "", 2), rule("obs64", "stream", "scenes", 1)];
        assert_eq!(auto_switch_target(&rules, &profiles(), "stream", 0, "OBS64.EXE", "", false), Some(("stream".into(), 1)));
        assert_eq!(auto_switch_target(&rules, &profiles(), "edit", 0, "OBS64.EXE", "", false), None);
    }

    #[test]
    fn page_rule_in_another_profile_cannot_pull_the_current_profile_away() {
        let rules = [rule("spotify.exe", "stream", "scenes", 1)];
        assert_eq!(auto_switch_target(&rules, &profiles(), "edit", 0, "spotify.exe", "", false), None);
    }

    #[test]
    fn profile_switch_opens_matching_local_page_in_one_step() {
        let rules = [
            rule("spotify.exe", "stream", "", 1),
            rule("spotify.exe", "stream", "scenes", 2),
        ];
        assert_eq!(auto_switch_target(&rules, &profiles(), "edit", 0, "spotify.exe", "", false), Some(("stream".into(), 1)));
        assert_eq!(auto_switch_target(&rules, &profiles(), "stream", 1, "spotify.exe", "", false), None);
    }

    #[test]
    fn spotify_page_rules_stay_local_to_each_profile() {
        let profiles = [profile("stream", &["main", "music"]), profile("gaming", &["main", "music"]), profile("work", &["main"])];
        let rules = [
            rule("spotify.exe", "gaming", "", 1),
            rule("spotify.exe", "stream", "music", 2),
            rule("spotify.exe", "gaming", "music", 3),
        ];
        assert_eq!(auto_switch_target(&rules, &profiles, "stream", 0, "spotify.exe", "", false), Some(("stream".into(), 1)));
        assert_eq!(auto_switch_target(&rules, &profiles, "gaming", 0, "spotify.exe", "", false), Some(("gaming".into(), 1)));
        assert_eq!(auto_switch_target(&rules, &profiles, "work", 0, "spotify.exe", "", false), Some(("gaming".into(), 1)));
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
            pages: vec![ProfilePage { id: format!("{id}-p"), name: id.into(), keys: vec![], touch_zones: vec![], encoders: vec![], strip_media: None, knob_led_effect: String::new() }],
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
