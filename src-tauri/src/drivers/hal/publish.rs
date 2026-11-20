use super::catalog::{now_ms, DeviceDescriptor, DiagnosticEntry, SyncKeyResult};
use super::io::{debug_flag, deck_write, deck_write_steps, note_push};
use crate::drivers::akp05e_protocol::{
    akp05e_key_image_slot, akp05e_touch_zone_slot, build_brightness_packet, build_button_mode_packet,
    build_screen_on_packet, build_touch_strip_packets, encode_procedural_key_jpeg, encode_touch_strip_jpeg,
    parse_hex_color,
};
use crate::drivers::driver::driver_for_device;
use crate::drivers::frame_cache::AntiFreezeFrameCache;
use crate::drivers::scheduler::IoStep;
use crate::media::{pace_delay, MediaStore, RenderedFrames, ZonePaint};
use crate::LockExt;
use std::time::Instant;

/// Applies a profile's key images and touch strip to the target device using SHA-256 frame deduplication.
pub fn sync_keys_to_hardware(
    dev: &DeviceDescriptor,
    key_colors_and_titles: &[(u8, String, String, String, String)], // (key_idx_0based, title, icon_name, accent_hex, bg_hex)
    touch_zones: &[(String, String, String)], // (title, value_text, accent_hex)
    frame_cache: &mut AntiFreezeFrameCache,
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> SyncKeyResult {
    let _display = crate::DISPLAY_WRITE.lock_safe();
    let keys: Vec<_> = key_colors_and_titles.iter().map(|(index, title, icon, accent, bg)| ScreenJob {
        index: *index, title: title.clone(), show_title: true, icon_name: icon.clone(),
        accent: accent.clone(), bg: bg.clone(), media_id: None, fit: "cover".into(),
        speed: 1.0, loop_playback: false, level: None, art: None,
    }).collect();
    let zones: Vec<_> = touch_zones.iter().map(|(label, value, accent)| ZoneJob {
        label: label.clone(), value_text: value.clone(), accent: accent.clone(),
        media_id: None, fit: "cover".into(), speed: 1.0, loop_playback: false,
    }).collect();
    let mut outcome = publish_screens(dev, &keys, &zones, None, &MediaStore::default(), frame_cache, diagnostics);
    let started = Instant::now();
    let result = write_animation_frames(dev, &outcome.bursts);
    let elapsed = started.elapsed().as_millis() as u64;
    frame_cache.finish_transfer(&dev.id, result.is_ok(), outcome.result.bytes_transmitted as u64, elapsed);
    outcome.result.elapsed_ms = elapsed;
    outcome.result.status_message = format!("Sent {} changed keys in {elapsed}ms", outcome.result.keys_transmitted);
    if let Err(err) = result {
        outcome.result.keys_transmitted = 0;
        outcome.result.bytes_transmitted = 0;
        outcome.result.status_message = format!("Layout transfer failed: {err}");
    }
    outcome.result
}

#[derive(Debug, Clone)]
pub struct ScreenJob {
    pub index: u8,
    pub title: String,
    pub show_title: bool,
    pub icon_name: String,
    pub accent: String,
    pub bg: String,
    pub media_id: Option<String>,
    pub fit: String,
    pub speed: f32,
    pub loop_playback: bool,
    pub level: Option<u8>,
    /// Album art to draw instead of the icon.
    pub art: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct ZoneJob {
    pub label: String,
    pub value_text: String,
    pub accent: String,
    pub media_id: Option<String>,
    pub fit: String,
    pub speed: f32,
    pub loop_playback: bool,
}

#[derive(Debug, Clone)]
pub struct StripJob {
    pub media_id: String,
    pub fit: String,
    pub speed: f32,
    pub loop_playback: bool,
}

#[derive(Debug, Clone)]
pub struct HidBurst {
    pub reports: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub enum PlaybackTarget {
    Key(u8),
    TouchZone(u8),
    Strip { width: u16, height: u16 },
}

/// Each AKP05E touch window is 176×112. The four of them sit under the keys,
/// with gaps the per-window image command cannot paint.
pub const AKP05E_TOUCH_WINDOW_W: u32 = 176;
pub const AKP05E_TOUCH_WINDOW_H: u32 = 112;

/// AKP05E / AKP05E Pro share one scrambled face. Other decks keep visual order.
pub fn device_uses_akp05e_face(dev: &DeviceDescriptor) -> bool {
    dev.vid == 0x0300
        && matches!(dev.pid, 0x3013 | 0x3004 | 0x3006 | 0x3014)
        && dev.keys.rows == 2
        && dev.keys.cols == 5
        && dev.touch_strip.is_some()
}

/// The single live HID connection still opens VID 0300:PID 3013 only. Do not
/// accidentally send a selected, unverified device's command to that handle.
pub(crate) fn ensure_akp_io_target(dev: &DeviceDescriptor) -> Result<(), String> {
    if dev.vid == 0x0300 && dev.pid == 0x3013 && dev.id == "ajazz-akp05e-pro" {
        Ok(())
    } else {
        Err(format!("{} is not yet supported by the live HID connection", dev.model_name))
    }
}

pub fn key_image_slot(dev: &DeviceDescriptor, visual_index: u8) -> u8 {
    if device_uses_akp05e_face(dev) {
        akp05e_key_image_slot(visual_index)
    } else {
        visual_index.saturating_add(1).max(1)
    }
}

pub fn touch_zone_image_slot(dev: &DeviceDescriptor, zone_index: u8) -> u8 {
    if device_uses_akp05e_face(dev) {
        akp05e_touch_zone_slot(zone_index)
    } else {
        zone_index.saturating_add(1).max(1)
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackSlot {
    pub slot_id: String,
    pub target: PlaybackTarget,
    pub jpegs: Vec<Vec<u8>>,
    pub delays_ms: Vec<u64>,
    pub loop_playback: bool,
    pub frame_index: usize,
    pub next_due_ms: u64,
}

pub struct PublishOutcome {
    pub result: SyncKeyResult,
    pub bursts: Vec<HidBurst>,
    pub playback: Vec<PlaybackSlot>,
}

fn packets_of(reports: Vec<[u8; 1025]>) -> HidBurst {
    HidBurst {
        reports: reports.into_iter().map(|r| r.to_vec()).collect(),
    }
}

/// Build key image reports with the selected device protocol. The live I/O worker
/// currently opens only the verified AKP05E, but rendering no longer hardcodes its framing.
pub fn key_image_burst(dev: &DeviceDescriptor, slot: u8, jpeg: &[u8]) -> HidBurst {
    HidBurst { reports: driver_for_device(dev).build_key_image_packets(slot, jpeg) }
}

fn queue_playback(
    playback: &mut Vec<PlaybackSlot>,
    slot_id: String,
    target: PlaybackTarget,
    rendered: &RenderedFrames,
) {
    if rendered.jpegs.len() < 2 {
        return;
    }
    let first_delay = rendered.delays_ms.first().copied().unwrap_or(100);
    playback.push(PlaybackSlot {
        slot_id,
        target,
        jpegs: rendered.jpegs.clone(),
        delays_ms: rendered.delays_ms.clone(),
        loop_playback: rendered.loop_playback,
        frame_index: 1,
        next_due_ms: now_ms().saturating_add(first_delay),
    });
}

/// Fits stills and GIFs to this device's key screens and touch strip, then builds the USB bursts.
/// Frame 0 is sent immediately. Later GIF frames are returned as playback slots.
pub fn publish_screens(
    dev: &DeviceDescriptor,
    keys: &[ScreenJob],
    zones: &[ZoneJob],
    strip: Option<&StripJob>,
    media: &MediaStore,
    frame_cache: &mut AntiFreezeFrameCache,
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> PublishOutcome {
    let start = Instant::now();
    let ts = now_ms();
    let mut bursts = Vec::new();
    let mut playback = Vec::new();
    let mut keys_transmitted = 0usize;
    let mut keys_skipped = 0usize;
    let mut bytes_sent = 0usize;
    let mut animated = 0usize;

    for job in keys {
        let bg = parse_hex_color(&job.bg, [21, 25, 37]);
        let title = if job.show_title { Some(job.title.as_str()) } else { None };
        let rendered = job.media_id.as_ref().and_then(|id| {
            media.render_fitted(
                id,
                dev.keys.pixel_width,
                dev.keys.pixel_height,
                dev.keys.rotation,
                if job.fit.is_empty() { "cover" } else { &job.fit },
                bg,
                title,
                if job.speed <= 0.0 { 1.0 } else { job.speed },
                job.loop_playback,
            )
        });
        let rendered = rendered.or_else(|| {
            let art = job.art.as_ref()?;
            let jpeg = crate::media::render_art_key(art, dev.keys.pixel_width, dev.keys.pixel_height, dev.keys.rotation, title)?;
            Some(RenderedFrames { jpegs: vec![jpeg], delays_ms: vec![pace_delay(100, 1.0)], loop_playback: false })
        });
        let rendered = if let Some(rendered) = rendered {
            rendered
        } else {
            if job.media_id.is_some() {
                diagnostics.push(DiagnosticEntry {
                    id: format!("diag-media-missing-{}", ts + job.index as u64),
                    timestamp_ms: ts,
                    level: "warning".to_string(),
                    device_id: dev.id.clone(),
                    stage: "MEDIA_MISSING".to_string(),
                    message: format!(
                        "Key #{} image is missing from the library. Upload it again.",
                        job.index + 1
                    ),
                    packet_hex: None,
                });
            }
            let jpeg = encode_procedural_key_jpeg(
                dev.keys.pixel_width,
                dev.keys.pixel_height,
                &job.accent,
                &job.bg,
                &job.icon_name,
                if job.show_title { job.title.as_str() } else { "" },
                job.index,
                dev.keys.rotation,
                job.level,
            );
            RenderedFrames {
                jpegs: vec![jpeg],
                delays_ms: vec![pace_delay(100, 1.0)],
                loop_playback: false,
            }
        };

        if rendered.jpegs.len() > 1 {
            animated += 1;
        }
        let frame0 = rendered.jpegs.first().cloned().unwrap_or_default();
        let mut hash_extra = Vec::with_capacity(job.title.len() + job.icon_name.len() + job.media_id.as_ref().map(|id| id.len() + job.fit.len()).unwrap_or(0));
        hash_extra.extend_from_slice(job.title.as_bytes());
        hash_extra.extend_from_slice(job.icon_name.as_bytes());
        if let Some(id) = &job.media_id {
            hash_extra.extend_from_slice(id.as_bytes());
            hash_extra.extend_from_slice(job.fit.as_bytes());
        }
        let slot_id = format!("key-{}", job.index);
        if frame_cache
            .should_transmit_parts(&dev.id, &slot_id, &[&frame0, &hash_extra])
            .is_some()
        {
            let burst = key_image_burst(dev, key_image_slot(dev, job.index), &frame0);
            bytes_sent += burst.reports.iter().map(Vec::len).sum::<usize>();
            bursts.push(burst);
            keys_transmitted += 1;
        } else {
            keys_skipped += 1;
        }
        queue_playback(
            &mut playback,
            slot_id,
            PlaybackTarget::Key(job.index),
            &rendered,
        );
    }

    let mut strip_note = "Touch strip unchanged.".to_string();
    if device_uses_akp05e_face(dev) {
        if let Some(ts_caps) = &dev.touch_strip {
            let windows = touch_window_count(ts_caps.zones, zones.len(), strip.is_some());
            let mut sent = 0usize;
            for i in 0..windows {
                let Some(rendered) = render_touch_window(
                    dev,
                    media,
                    zones.get(i),
                    strip,
                    i,
                    windows,
                    ts_caps.pixel_width,
                    ts_caps.pixel_height,
                ) else {
                    continue;
                };
                if rendered.jpegs.len() > 1 {
                    animated += 1;
                }
                let frame0 = rendered.jpegs.first().cloned().unwrap_or_default();
                let slot_id = format!("touch-zone-{i}");
                if frame_cache
                    .should_transmit(&dev.id, &slot_id, &frame0)
                    .is_some()
                {
                    let burst = key_image_burst(dev, touch_zone_image_slot(dev, i as u8), &frame0);
                    bytes_sent += burst.reports.iter().map(Vec::len).sum::<usize>();
                    bursts.push(burst);
                    sent += 1;
                }
                queue_playback(
                    &mut playback,
                    slot_id,
                    PlaybackTarget::TouchZone(i as u8),
                    &rendered,
                );
            }
            if windows > 0 {
                strip_note = format!("Touch strip sent as {sent} windows under the keys.");
            }
        }
    } else if let Some(ts_caps) = &dev.touch_strip {
        let zone_tuples: Vec<(String, String, String)> = zones
            .iter()
            .map(|z| (z.label.clone(), z.value_text.clone(), z.accent.clone()))
            .collect();
        let rendered = if let Some(job) = strip {
            media.render_fitted(
                &job.media_id,
                ts_caps.pixel_width,
                ts_caps.pixel_height,
                dev.keys.rotation,
                if job.fit.is_empty() { "cover" } else { &job.fit },
                [13, 16, 24],
                None,
                if job.speed <= 0.0 { 1.0 } else { job.speed },
                job.loop_playback,
            )
        } else if zones.iter().any(|z| z.media_id.is_some()) {
            let paints: Vec<ZonePaint<'_>> = zones
                .iter()
                .map(|z| ZonePaint {
                    label: &z.label,
                    value: &z.value_text,
                    accent: &z.accent,
                    media_id: z.media_id.as_deref(),
                    fit: if z.fit.is_empty() { "cover" } else { &z.fit },
                    speed: if z.speed <= 0.0 { 1.0 } else { z.speed },
                    loop_playback: z.loop_playback,
                })
                .collect();
            Some(media.render_zone_strip(
                ts_caps.pixel_width,
                ts_caps.pixel_height,
                dev.keys.rotation,
                &paints,
            ))
        } else if !zone_tuples.is_empty() {
            let jpeg = encode_touch_strip_jpeg(
                ts_caps.pixel_width,
                ts_caps.pixel_height,
                &zone_tuples,
                dev.keys.rotation,
            );
            Some(RenderedFrames {
                jpegs: vec![jpeg],
                delays_ms: vec![100],
                loop_playback: false,
            })
        } else {
            None
        };

        if let Some(rendered) = rendered {
            if rendered.jpegs.len() > 1 {
                animated += 1;
            }
            let frame0 = rendered.jpegs.first().cloned().unwrap_or_default();
            if frame_cache
                .should_transmit(&dev.id, "touch-strip", &frame0)
                .is_some()
            {
                let burst = packets_of(build_touch_strip_packets(
                    &frame0,
                    0,
                    0,
                    ts_caps.pixel_width as u16,
                    ts_caps.pixel_height as u16,
                ));
                bytes_sent += burst.reports.len() * 1025;
                bursts.push(burst);
                strip_note = format!(
                    "Touch strip {}×{} updated.",
                    ts_caps.pixel_width, ts_caps.pixel_height
                );
            } else {
                strip_note = "Touch strip unchanged.".to_string();
            }
            queue_playback(
                &mut playback,
                "touch-strip".to_string(),
                PlaybackTarget::Strip {
                    width: ts_caps.pixel_width as u16,
                    height: ts_caps.pixel_height as u16,
                },
                &rendered,
            );
        }
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let status_message = format!(
        "Prepared {}/{} screens for {} ({}×{}px, {}°). {} animation{}. {}",
        keys_transmitted,
        keys.len(),
        dev.model_name,
        dev.keys.pixel_width,
        dev.keys.pixel_height,
        dev.keys.rotation,
        animated,
        if animated == 1 { "" } else { "s" },
        strip_note
    );
    diagnostics.push(DiagnosticEntry {
        id: format!("diag-publish-{ts}"),
        timestamp_ms: ts,
        level: "info".to_string(),
        device_id: dev.id.clone(),
        stage: "SYNC_PROFILE_KEYS".to_string(),
        message: status_message.clone(),
        packet_hex: Some(format!(
            "CRT..BAT ({keys_transmitted} screens, {keys_skipped} cached, {animated} animations, {bytes_sent} bytes)"
        )),
    });

    PublishOutcome {
        result: SyncKeyResult {
            device_id: dev.id.clone(),
            connected_physical: dev.connected,
            keys_evaluated: keys.len(),
            keys_transmitted,
            keys_deduplicated_skipped: keys_skipped,
            bytes_transmitted: bytes_sent,
            elapsed_ms,
            status_message,
        },
        bursts,
        playback,
    }
}

pub fn write_hid_bursts(dev: &DeviceDescriptor, bursts: &[HidBurst]) -> Result<(), String> {
    write_reports(dev, bursts, true)
}

/// Animation frames must not repeat the handshake. That command switches the backlight off.
pub fn write_animation_frames(dev: &DeviceDescriptor, bursts: &[HidBurst]) -> Result<(), String> {
    write_reports(dev, bursts, false)
}

pub(crate) fn panel_brightness(dev: &DeviceDescriptor) -> u8 {
    match dev.brightness {
        0 => 95,
        percent => percent.min(100),
    }
}

fn write_reports(
    dev: &DeviceDescriptor,
    bursts: &[HidBurst],
    session_setup: bool,
) -> Result<(), String> {
    if !dev.connected || dev.is_simulator {
        return Err("Deck is not connected".into());
    }
    ensure_akp_io_target(dev)?;
    if bursts.is_empty() { return Ok(()); }
    // DIS is only for a full connect. Ajazz names that packet disconnect, and
    // sending it before a one-key update clears every icon that was not in the update.
    // HAN is sleep, and sending it is what made a press go black.
    let brightness = panel_brightness(dev);
    let lig = build_brightness_packet(brightness);
    let mut steps: Vec<IoStep> = Vec::new();
    if session_setup {
        // DIS resets the panel. Give it time to finish before anything else arrives. The
        // `no-dis` marker file skips it for comparison. The black panels that blamed this
        // packet were the official Stream Dock app fighting for the deck, not DIS itself.
        // A deck that has just rebooted drops pictures that arrive while it is still
        // finishing the reset: about half of the keys stayed blank with a 400 ms wait.
        if !debug_flag("no-dis") {
            steps.push(IoStep::command(build_screen_on_packet().to_vec(), 1200));
        }
        steps.push(IoStep::command(build_button_mode_packet().to_vec(), 150));
        steps.push(IoStep::command(lig.to_vec(), 150));
    }
    // A full push sends every picture back to back. Give the firmware a moment to decode
    // each one before the next header arrives.
    let after_picture: u64 = if session_setup { 45 } else { 6 };
    for burst in bursts {
        let last = burst.reports.len().saturating_sub(1);
        for (i, report) in burst.reports.iter().enumerate() {
            steps.push(IoStep { report: report.clone(),
                pause_ms: if i == last { after_picture } else { 0 }, boundary_after: i == last });
        }
    }
    if session_setup {
        // The backlight stayed off after a push on a deck that was already running: the
        // firmware ignores a brightness that matches the one it holds. A different value
        // first forces it to apply the real one.
        steps.push(IoStep::command(build_brightness_packet(nudged_brightness(brightness)).to_vec(), 200));
    }
    steps.push(IoStep::command(lig.to_vec(), 0));
    let result = deck_write_steps(steps);
    note_push(&result, session_setup);
    result
}

/// A brightness close to `percent` but not equal to it.
pub(crate) fn nudged_brightness(percent: u8) -> u8 {
    if percent > 8 {
        percent - 6
    } else {
        percent + 6
    }
}

pub fn write_one_report(dev: &DeviceDescriptor, report: &[u8]) -> Result<(), String> {
    if !dev.connected || dev.is_simulator {
        return Err("Deck is not connected".into());
    }
    ensure_akp_io_target(dev)?;
    if report.is_empty() { return Ok(()); }
    deck_write(vec![(report.to_vec(), 0)])
}

/// Clears a single key (`0x01..=0x20`) or all keys (`0xFF`) on the device.
fn touch_window_count(cap: u8, zone_len: usize, has_strip: bool) -> usize {
    let cap = cap.max(1).min(4) as usize;
    if has_strip {
        cap
    } else {
        zone_len.min(cap)
    }
}

fn render_touch_window(
    dev: &DeviceDescriptor,
    media: &MediaStore,
    zone: Option<&ZoneJob>,
    strip: Option<&StripJob>,
    index: usize,
    count: usize,
    strip_width: u32,
    strip_height: u32,
) -> Option<RenderedFrames> {
    let rotation = dev.keys.rotation;
    if let Some(job) = strip {
        if let Some(frames) = media.render_strip_window(
            &job.media_id,
            strip_width,
            strip_height,
            index as u32,
            count as u32,
            AKP05E_TOUCH_WINDOW_W,
            AKP05E_TOUCH_WINDOW_H,
            rotation,
            if job.fit.is_empty() { "cover" } else { &job.fit },
            if job.speed <= 0.0 { 1.0 } else { job.speed },
            job.loop_playback,
        ) {
            return Some(frames);
        }
    }
    let zone = zone?;
    if let Some(id) = &zone.media_id {
        if let Some(frames) = media.render_fitted(
            id,
            AKP05E_TOUCH_WINDOW_W,
            AKP05E_TOUCH_WINDOW_H,
            rotation,
            if zone.fit.is_empty() { "cover" } else { &zone.fit },
            [13, 16, 24],
            Some(zone.label.as_str()),
            if zone.speed <= 0.0 { 1.0 } else { zone.speed },
            zone.loop_playback,
        ) {
            return Some(frames);
        }
    }
    let jpeg = encode_touch_strip_jpeg(
        AKP05E_TOUCH_WINDOW_W,
        AKP05E_TOUCH_WINDOW_H,
        &[(zone.label.clone(), zone.value_text.clone(), zone.accent.clone())],
        rotation,
    );
    Some(RenderedFrames {
        jpegs: vec![jpeg],
        delays_ms: vec![100],
        loop_playback: false,
    })
}
