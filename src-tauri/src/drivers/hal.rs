use crate::drivers::akp05e_protocol::{
    akp05e_key_image_slot, akp05e_touch_zone_slot, build_brightness_packet, build_clear_key_packet,
    build_button_mode_packet, build_heartbeat_packet, build_key_image_packets, build_screen_on_packet, build_touch_strip_packets,
    encode_procedural_key_jpeg, encode_touch_strip_jpeg, parse_hex_color,
};
use crate::drivers::frame_cache::{AntiFreezeFrameCache, FrameCacheStats};
use crate::media::{pace_delay, MediaStore, RenderedFrames, ZonePaint};
use hidapi::HidApi;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyGridCapabilities {
    pub rows: u8,
    pub cols: u8,
    pub count: u8,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub rotation: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchStripCapabilities {
    pub zones: u8,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub supports_swipe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncoderCapabilities {
    pub count: u8,
    pub supports_push: bool,
    pub detents_per_revolution: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDescriptor {
    pub id: String,
    pub vendor_name: String,
    pub model_name: String,
    pub vid: u16,
    pub pid: u16,
    pub vid_pid_hex: String,
    pub driver_name: String,
    pub protocol_family: String,
    pub verified_driver: bool,
    pub connected: bool,
    pub is_simulator: bool,
    pub serial_number: String,
    pub firmware_version: String,
    pub hid_path: Option<String>,
    pub assigned_profile_id: String,
    pub brightness: u8,
    pub keys: KeyGridCapabilities,
    pub touch_strip: Option<TouchStripCapabilities>,
    pub encoders: Option<EncoderCapabilities>,
    pub supports_brightness: bool,
    pub last_seen_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEntry {
    pub id: String,
    pub timestamp_ms: u64,
    pub level: String, // "info" | "success" | "warning" | "error"
    pub device_id: String,
    pub stage: String,
    pub message: String,
    pub packet_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncKeyResult {
    pub device_id: String,
    pub connected_physical: bool,
    pub keys_evaluated: usize,
    pub keys_transmitted: usize,
    pub keys_deduplicated_skipped: usize,
    pub bytes_transmitted: usize,
    pub elapsed_ms: u64,
    pub status_message: String,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Returns the catalog of supported and simulated hardware models.
pub fn builtin_device_catalog() -> Vec<DeviceDescriptor> {
    let ts = now_ms();
    vec![
        DeviceDescriptor {
            id: "ajazz-akp05e-pro".to_string(),
            vendor_name: "AJAZZ".to_string(),
            model_name: "Ajazz AKP05E Pro".to_string(),
            vid: 0x0300,
            pid: 0x3013,
            vid_pid_hex: "0300:3013".to_string(),
            driver_name: "CrtHidDriver v1.0 (Verified 1024B Disassembly)".to_string(),
            protocol_family: "CRT-1024B-HID".to_string(),
            verified_driver: true,
            connected: false,
            is_simulator: false,
            serial_number: "Not detected".to_string(),
            firmware_version: "V4.AKP05E Pro.02.010".to_string(),
            hid_path: None,
            assigned_profile_id: "profile-streaming".to_string(),
            brightness: 95,
            keys: KeyGridCapabilities {
                rows: 2,
                cols: 5,
                count: 10,
                pixel_width: 112,
                pixel_height: 112,
                rotation: 180,
            },
            touch_strip: Some(TouchStripCapabilities {
                zones: 4,
                pixel_width: 800,
                pixel_height: 100,
                supports_swipe: true,
            }),
            encoders: Some(EncoderCapabilities {
                count: 4,
                supports_push: true,
                detents_per_revolution: 24,
            }),
            supports_brightness: true,
            last_seen_ms: ts,
        },
        DeviceDescriptor {
            id: "ajazz-akp153".to_string(),
            vendor_name: "AJAZZ".to_string(),
            model_name: "Ajazz AKP153 (15 Keys)".to_string(),
            vid: 0x0300,
            pid: 0x1010,
            vid_pid_hex: "0300:1010".to_string(),
            driver_name: "CrtHidDriver v1.0 (CRT Family)".to_string(),
            protocol_family: "CRT-64B-HID".to_string(),
            verified_driver: false,
            connected: false,
            is_simulator: true,
            serial_number: "SIM-AKP153".to_string(),
            firmware_version: "Simulator Profile".to_string(),
            hid_path: None,
            assigned_profile_id: "profile-streaming".to_string(),
            brightness: 80,
            keys: KeyGridCapabilities {
                rows: 3,
                cols: 5,
                count: 15,
                pixel_width: 85,
                pixel_height: 85,
                rotation: 90,
            },
            touch_strip: None,
            encoders: None,
            supports_brightness: true,
            last_seen_ms: ts,
        },
        DeviceDescriptor {
            id: "elgato-mk2".to_string(),
            vendor_name: "Elgato".to_string(),
            model_name: "Elgato Stream Deck MK.2".to_string(),
            vid: 0x0FD9,
            pid: 0x0080,
            vid_pid_hex: "0FD9:0080".to_string(),
            driver_name: "ElgatoHidDriver v1.0 (Report 0x02)".to_string(),
            protocol_family: "ELGATO-HID-V2".to_string(),
            verified_driver: false,
            connected: false,
            is_simulator: true,
            serial_number: "SIM-ELGATO-MK2".to_string(),
            firmware_version: "Simulator Profile".to_string(),
            hid_path: None,
            assigned_profile_id: "profile-streaming".to_string(),
            brightness: 80,
            keys: KeyGridCapabilities {
                rows: 3,
                cols: 5,
                count: 15,
                pixel_width: 72,
                pixel_height: 72,
                rotation: 0,
            },
            touch_strip: None,
            encoders: None,
            supports_brightness: true,
            last_seen_ms: ts,
        },
        DeviceDescriptor {
            id: "mirabox-n3".to_string(),
            vendor_name: "MiraBox / AJAZZ".to_string(),
            model_name: "MiraBox N3 / Ajazz AKP03".to_string(),
            vid: 0x0300,
            pid: 0x1003,
            vid_pid_hex: "0300:1003".to_string(),
            driver_name: "CrtHidDriver v1.0 (CRT Family)".to_string(),
            protocol_family: "CRT-64B-HID".to_string(),
            verified_driver: false,
            connected: false,
            is_simulator: true,
            serial_number: "SIM-N3-KNOB".to_string(),
            firmware_version: "Simulator Profile".to_string(),
            hid_path: None,
            assigned_profile_id: "profile-editing".to_string(),
            brightness: 85,
            keys: KeyGridCapabilities {
                rows: 2,
                cols: 3,
                count: 6,
                pixel_width: 72,
                pixel_height: 72,
                rotation: 0,
            },
            touch_strip: None,
            encoders: Some(EncoderCapabilities {
                count: 3,
                supports_push: true,
                detents_per_revolution: 24,
            }),
            supports_brightness: true,
            last_seen_ms: ts,
        },
        DeviceDescriptor {
            id: "elgato-xl".to_string(),
            vendor_name: "Elgato".to_string(),
            model_name: "Elgato Stream Deck XL (32 Keys)".to_string(),
            vid: 0x0FD9,
            pid: 0x006C,
            vid_pid_hex: "0FD9:006C".to_string(),
            driver_name: "ElgatoHidDriver v1.0 (Report 0x02)".to_string(),
            protocol_family: "ELGATO-HID-V2".to_string(),
            verified_driver: false,
            connected: false,
            is_simulator: true,
            serial_number: "SIM-ELGATO-XL".to_string(),
            firmware_version: "Simulator Profile".to_string(),
            hid_path: None,
            assigned_profile_id: "profile-work".to_string(),
            brightness: 80,
            keys: KeyGridCapabilities {
                rows: 4,
                cols: 8,
                count: 32,
                pixel_width: 96,
                pixel_height: 96,
                rotation: 0,
            },
            touch_strip: None,
            encoders: None,
            supports_brightness: true,
            last_seen_ms: ts,
        },
    ]
}

/// Scans physical USB HID devices via `hidapi` and updates connection states.
pub fn scan_usb_devices(
    catalog: &mut [DeviceDescriptor],
    diagnostics: &mut Vec<DiagnosticEntry>,
) {
    let ts = now_ms();
    match HidApi::new() {
        Ok(api) => {
            for dev in catalog.iter_mut() {
                dev.connected = false;
                dev.hid_path = None;
            }

            for info in api.device_list() {
                let vid = info.vendor_id();
                let pid = info.product_id();
                let path_str = info.path().to_string_lossy().to_string();

                for dev in catalog.iter_mut() {
                    if dev.vid == vid && dev.pid == pid {
                        // Prefer MI_00 or vendor-defined usage page (0xFFA0 / 0xFF00 / interface 0)
                        let is_preferred = dev.hid_path.is_none()
                            || path_str.to_lowercase().contains("mi_00")
                            || info.interface_number() == 0;
                        if is_preferred {
                            dev.connected = true;
                            dev.is_simulator = false;
                            dev.hid_path = Some(path_str.clone());
                            dev.last_seen_ms = ts;
                            if let Some(sn) = info.serial_number() {
                                if !sn.trim().is_empty() {
                                    dev.serial_number = sn.trim().to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(err) => {
            diagnostics.push(DiagnosticEntry {
                id: format!("diag-{}", ts),
                timestamp_ms: ts,
                level: "error".to_string(),
                device_id: "system".to_string(),
                stage: "USB_SCAN".to_string(),
                message: format!("HIDAPI initialization failed: {}", err),
                packet_hex: None,
            });
        }
    }
}

/// Sends a real `CRT\0\0LIG` brightness packet to a connected CRT-family device (`0300:3013`).
pub fn send_hardware_brightness(
    dev: &DeviceDescriptor,
    percent: u8,
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> Result<String, String> {
    let ts = now_ms();
    let pkt = build_brightness_packet(percent);
    let hex_preview = pkt[1..13]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    if !dev.connected {
        diagnostics.push(DiagnosticEntry {
            id: format!("diag-{}", ts),
            timestamp_ms: ts,
            level: "info".to_string(),
            device_id: dev.id.clone(),
            stage: "SIM_BRIGHTNESS".to_string(),
            message: format!("Simulated brightness update to {}% ({})", percent, dev.model_name),
            packet_hex: Some(hex_preview),
        });
        return Ok(format!("Simulator brightness set to {}%", percent));
    }

    // Brightness is LIG only. A handshake (`CRT HAN`) switches the backlight off,
    // which is the fade-to-black the deck shows after an ordinary control.
    deck_write(vec![(pkt.to_vec(), 0)])?;

    diagnostics.push(DiagnosticEntry {
        id: format!("diag-{}", ts),
        timestamp_ms: ts,
        level: "success".to_string(),
        device_id: dev.id.clone(),
        stage: "CRT_LIG_BRIGHTNESS".to_string(),
        message: format!("Applied hardware brightness {}% to {}", percent, dev.model_name),
        packet_hex: Some(hex_preview),
    });

    Ok(format!("Hardware brightness set to {}% on {}", percent, dev.model_name))
}

/// Turns the panel on and restores the backlight. Runs on the deck's single
/// connection, right after the click that was read from it.
pub fn wake_panel(dev: &DeviceDescriptor) -> Result<(), String> {
    if !dev.connected || debug_flag("no-wake") {
        return Ok(());
    }
    // Brightness only. DIS clears every picture and leaves the backlight off.
    let brightness = panel_brightness(dev);
    deck_write(vec![
        (build_brightness_packet(nudged_brightness(brightness)).to_vec(), 120),
        (build_brightness_packet(brightness).to_vec(), 0),
    ])
}

/// Sets the backlight without putting the panel to sleep.
pub fn write_panel_brightness(dev: &DeviceDescriptor, percent: u8) -> Result<(), String> {
    if !dev.connected {
        return Ok(());
    }
    let pkt = build_brightness_packet(percent.min(100));
    deck_write(vec![(pkt.to_vec(), 0)])
}

/// Applies a profile's key images and touch strip to the target device using SHA-256 frame deduplication.
pub fn sync_keys_to_hardware(
    dev: &DeviceDescriptor,
    key_colors_and_titles: &[(u8, String, String, String, String)], // (key_idx_0based, title, icon_name, accent_hex, bg_hex)
    touch_zones: &[(String, String, String)], // (title, value_text, accent_hex)
    frame_cache: &mut AntiFreezeFrameCache,
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> SyncKeyResult {
    let start = Instant::now();
    let ts = now_ms();

    let mut keys_transmitted = 0usize;
    let mut keys_skipped = 0usize;
    let mut bytes_sent = 0usize;

    let api_opt = if dev.connected { HidApi::new().ok() } else { None };
    let handle_opt = api_opt
        .as_ref()
        .and_then(|api| open_crt_device(api, dev).ok());

    if let Some(ref h) = handle_opt {
        let _ = h.write(&build_screen_on_packet());
        let _ = h.write(&build_button_mode_packet());
        let lig = build_brightness_packet(dev.brightness.max(90));
        let _ = h.write(&lig);
        std::thread::sleep(std::time::Duration::from_millis(8));
    }

    for (idx, title, icon_name, accent, bg) in key_colors_and_titles {
        let jpeg = encode_procedural_key_jpeg(
            dev.keys.pixel_width,
            dev.keys.pixel_height,
            accent,
            bg,
            icon_name,
            title,
            *idx,
            180,
            None,
        );
        // Include title + icon + style in frame hash input so label/icon edits trigger a sync while identical keys are skipped
        let mut hash_input = jpeg.clone();
        hash_input.extend_from_slice(title.as_bytes());
        hash_input.extend_from_slice(icon_name.as_bytes());

        let slot_id = format!("key-{}", idx);
        if frame_cache
            .should_transmit(&dev.id, &slot_id, &hash_input)
            .is_some()
        {
            let pkts = build_key_image_packets(key_image_slot(dev, *idx), &jpeg);
            if let Some(ref h) = handle_opt {
                for pkt in &pkts {
                    let _ = h.write(pkt);
                }
                std::thread::sleep(std::time::Duration::from_millis(6));
            }
            keys_transmitted += 1;
            bytes_sent += pkts.len() * 1025;
        } else {
            keys_skipped += 1;
        }
    }

    // AKP05E touch zones are four image slots under the keys. A full-width
    // draw at y=0 lands on the top key row, so each zone is its own JPEG.
    if device_uses_akp05e_face(dev) && !touch_zones.is_empty() {
        for (i, zone) in touch_zones.iter().take(4).enumerate() {
            let jpeg = encode_touch_strip_jpeg(
                AKP05E_TOUCH_WINDOW_W,
                AKP05E_TOUCH_WINDOW_H,
                std::slice::from_ref(zone),
                dev.keys.rotation,
            );
            let slot_id = format!("touch-zone-{i}");
            if frame_cache
                .should_transmit(&dev.id, &slot_id, &jpeg)
                .is_some()
            {
                let pkts = build_key_image_packets(touch_zone_image_slot(dev, i as u8), &jpeg);
                if let Some(ref h) = handle_opt {
                    for pkt in &pkts {
                        let _ = h.write(pkt);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(6));
                }
                bytes_sent += pkts.len() * 1025;
            }
        }
    } else if let Some(ref ts_caps) = dev.touch_strip {
        if !touch_zones.is_empty() {
            let strip_jpeg = encode_touch_strip_jpeg(
                ts_caps.pixel_width,
                ts_caps.pixel_height,
                touch_zones,
                dev.keys.rotation,
            );
            if frame_cache
                .should_transmit(&dev.id, "touch-strip-800x100", &strip_jpeg)
                .is_some()
            {
                let strip_pkts = build_touch_strip_packets(
                    &strip_jpeg,
                    0,
                    0,
                    ts_caps.pixel_width as u16,
                    ts_caps.pixel_height as u16,
                );
                if let Some(ref h) = handle_opt {
                    for pkt in &strip_pkts {
                        let _ = h.write(pkt);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(8));
                }
                bytes_sent += strip_pkts.len() * 1025;
            }
        }
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    frame_cache.stats.last_transfer_latency_ms = elapsed_ms;

    let status_message = if handle_opt.is_some() {
        format!(
            "Synced {}/{} changed keys to physical {} in {}ms ({} unchanged keys skipped via SHA-256 cache)",
            keys_transmitted,
            key_colors_and_titles.len(),
            dev.model_name,
            elapsed_ms,
            keys_skipped
        )
    } else {
        format!(
            "Validated {}/{} changed keys in simulator ({}ms, {} unchanged keys skipped via SHA-256 cache)",
            keys_transmitted,
            key_colors_and_titles.len(),
            elapsed_ms,
            keys_skipped
        )
    };

    diagnostics.push(DiagnosticEntry {
        id: format!("diag-{}", ts),
        timestamp_ms: ts,
        level: if handle_opt.is_some() { "success" } else { "info" }.to_string(),
        device_id: dev.id.clone(),
        stage: "SYNC_PROFILE_KEYS".to_string(),
        message: status_message.clone(),
        packet_hex: Some(format!(
            "CRT..BAT ({} keys transmitted, {} deduplicated, {} bytes)",
            keys_transmitted, keys_skipped, bytes_sent
        )),
    });

    SyncKeyResult {
        device_id: dev.id.clone(),
        connected_physical: handle_opt.is_some(),
        keys_evaluated: key_colors_and_titles.len(),
        keys_transmitted,
        keys_deduplicated_skipped: keys_skipped,
        bytes_transmitted: bytes_sent,
        elapsed_ms,
        status_message,
    }
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
        let mut hash_input = frame0.clone();
        hash_input.extend_from_slice(job.title.as_bytes());
        hash_input.extend_from_slice(job.icon_name.as_bytes());
        if let Some(id) = &job.media_id {
            hash_input.extend_from_slice(id.as_bytes());
            hash_input.extend_from_slice(job.fit.as_bytes());
        }
        let slot_id = format!("key-{}", job.index);
        if frame_cache
            .should_transmit(&dev.id, &slot_id, &hash_input)
            .is_some()
        {
            let burst = packets_of(build_key_image_packets(
                key_image_slot(dev, job.index),
                &frame0,
            ));
            bytes_sent += burst.reports.len() * 1025;
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
                    let burst = packets_of(build_key_image_packets(
                        touch_zone_image_slot(dev, i as u8),
                        &frame0,
                    ));
                    bytes_sent += burst.reports.len() * 1025;
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
    frame_cache.stats.last_transfer_latency_ms = elapsed_ms;
    let status_message = format!(
        "Updated {}/{} screens on {} ({}×{}px, {}°). {} animation{}. {}",
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
        level: "success".to_string(),
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

fn panel_brightness(dev: &DeviceDescriptor) -> u8 {
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
    if !dev.connected || bursts.is_empty() {
        return Ok(());
    }
    // DIS is only for a full connect. Ajazz names that packet disconnect, and
    // sending it before a one-key update clears every icon that was not in the update.
    // HAN is sleep, and sending it is what made a press go black.
    let brightness = panel_brightness(dev);
    let lig = build_brightness_packet(brightness);
    let mut steps: Vec<(Vec<u8>, u64)> = Vec::new();
    if session_setup {
        // DIS resets the panel. Give it time to finish before anything else arrives. The
        // `no-dis` marker file skips it for comparison. The black panels that blamed this
        // packet were the official Stream Dock app fighting for the deck, not DIS itself.
        // A deck that has just rebooted drops pictures that arrive while it is still
        // finishing the reset: about half of the keys stayed blank with a 400 ms wait.
        if !debug_flag("no-dis") {
            steps.push((build_screen_on_packet().to_vec(), 1200));
        }
        steps.push((build_button_mode_packet().to_vec(), 150));
        steps.push((lig.to_vec(), 150));
    }
    // A full push sends every picture back to back. Give the firmware a moment to decode
    // each one before the next header arrives.
    let after_picture: u64 = if session_setup { 45 } else { 6 };
    for burst in bursts {
        let last = burst.reports.len().saturating_sub(1);
        for (i, report) in burst.reports.iter().enumerate() {
            steps.push((report.clone(), if i == last { after_picture } else { 0 }));
        }
    }
    if session_setup {
        // The backlight stayed off after a push on a deck that was already running: the
        // firmware ignores a brightness that matches the one it holds. A different value
        // first forces it to apply the real one.
        steps.push((build_brightness_packet(nudged_brightness(brightness)).to_vec(), 200));
    }
    steps.push((lig.to_vec(), 0));
    deck_write(steps)
}

/// A brightness close to `percent` but not equal to it.
fn nudged_brightness(percent: u8) -> u8 {
    if percent > 8 {
        percent - 6
    } else {
        percent + 6
    }
}

pub fn write_one_report(dev: &DeviceDescriptor, report: &[u8]) -> Result<(), String> {
    if !dev.connected || report.is_empty() {
        return Ok(());
    }
    deck_write(vec![(report.to_vec(), 0)])
}

/// Clears a single key (`0x01..=0x20`) or all keys (`0xFF`) on the device.
pub fn clear_hardware_key(
    dev: &DeviceDescriptor,
    key_idx_1based: u8,
    frame_cache: &mut AntiFreezeFrameCache,
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> Result<(String, Vec<u8>), String> {
    let ts = now_ms();
    frame_cache.invalidate_device(&dev.id);
    let packet_slot = if key_idx_1based == 0xFF {
        0xFF
    } else {
        key_image_slot(dev, key_idx_1based.saturating_sub(1))
    };
    let pkt = build_clear_key_packet(packet_slot);
    let hex_preview = pkt[1..14]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    diagnostics.push(DiagnosticEntry {
        id: format!("diag-{}", ts),
        timestamp_ms: ts,
        level: "info".to_string(),
        device_id: dev.id.clone(),
        stage: "CRT_CLE_KEY".to_string(),
        message: format!("Cleared key slot 0x{:02X} on {}", key_idx_1based, dev.model_name),
        packet_hex: Some(hex_preview),
    });

    Ok((format!("Cleared key 0x{:02X}", key_idx_1based), pkt.to_vec()))
}

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

// The deck is opened exactly once, by one thread. That thread reads button
// reports and performs every write that other threads queue. Opening the device
// from several places at once is what made the firmware reset, drop icons, and
// sleep the panel on a press.
struct DeckJob {
    steps: Vec<(Vec<u8>, u64)>,
    reply: mpsc::Sender<Result<(), String>>,
}

struct DeckIo {
    jobs: mpsc::Sender<DeckJob>,
    reports: Mutex<mpsc::Receiver<Vec<u8>>>,
}

static DECK_IO: OnceLock<DeckIo> = OnceLock::new();
static DECK_GENERATION: AtomicU64 = AtomicU64::new(0);

fn deck_io() -> &'static DeckIo {
    DECK_IO.get_or_init(|| {
        let (job_tx, job_rx) = mpsc::channel::<DeckJob>();
        let (report_tx, report_rx) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || deck_io_loop(job_rx, report_tx));
        DeckIo {
            jobs: job_tx,
            reports: Mutex::new(report_rx),
        }
    })
}

fn open_akp05e(api: &HidApi) -> Option<hidapi::HidDevice> {
    api.device_list().find_map(|info| {
        if info.vendor_id() != 0x0300 || info.product_id() != 0x3013 {
            return None;
        }
        let path = info.path().to_string_lossy().to_lowercase();
        if path.contains("mi_00") || info.interface_number() == 0 {
            info.open_device(api).ok()
        } else {
            None
        }
    })
}

/// Appends one line to `%APPDATA%\DeckForge\deck-io.log` so a drop or a black panel
/// can be matched to what the connection was doing.
fn deck_log(line: &str) {
    use std::io::Write;
    let Some(dir) = std::env::var_os("APPDATA") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("DeckForge");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("deck-io.log");
    if std::fs::metadata(&path).map(|m| m.len() > 1_000_000).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{} {}", now_ms(), line);
    }
}

fn debug_flag_path(name: &str) -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("APPDATA")?;
    Some(std::path::PathBuf::from(dir).join("DeckForge").join(name))
}

/// True while a marker file with this name exists in `%APPDATA%\DeckForge`.
fn debug_flag(name: &str) -> bool {
    debug_flag_path(name).map(|p| p.exists()).unwrap_or(false)
}

/// True once, then deletes the marker file.
fn debug_flag_take(name: &str) -> bool {
    match debug_flag_path(name) {
        Some(path) if path.exists() => {
            let _ = std::fs::remove_file(path);
            true
        }
        _ => false,
    }
}

/// Reads and deletes a marker file that carries text, or None when it is absent.
fn debug_file_take(name: &str) -> Option<String> {
    let path = debug_flag_path(name)?;
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(path);
    Some(text)
}

/// Runs a protocol experiment from `%APPDATA%\DeckForge\deck-cmd.txt`: space-separated
/// tokens such as `LIG95 CON W3000 MOD1`. Each result goes to `deck-io.log`. This is how
/// a wake sequence is tried on the real deck without rebuilding the app.
fn run_debug_script(handle: &hidapi::HidDevice, script: &str) {
    for token in script.split_whitespace() {
        let upper = token.to_ascii_uppercase();
        let result = if let Some(ms) = upper.strip_prefix('W').and_then(|v| v.parse::<u64>().ok()) {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            deck_log(&format!("debug wait {ms}ms"));
            continue;
        } else if let Some(percent) = upper.strip_prefix("LIG").and_then(|v| v.parse::<u8>().ok()) {
            handle.write(&build_brightness_packet(percent))
        } else {
            match upper.as_str() {
                "DIS" => handle.write(&build_screen_on_packet()),
                "MOD1" => handle.write(&build_button_mode_packet()),
                "CON" => handle.write(&build_heartbeat_packet()),
                "STP" => handle.write(&crate::drivers::akp05e_protocol::build_finish_packet()),
                "CLE" => handle.write(&build_clear_key_packet(0xFF)),
                "VER" => handle.write(&crate::drivers::akp05e_protocol::build_firmware_version_packet()),
                other => {
                    deck_log(&format!("debug unknown token {other}"));
                    continue;
                }
            }
        };
        deck_log(&format!("debug cmd {upper}: {result:?}"));
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
}

fn deck_io_loop(jobs: mpsc::Receiver<DeckJob>, reports: mpsc::Sender<Vec<u8>>) {
    let mut open: Option<(HidApi, hidapi::HidDevice)> = None;
    let mut errors = 0u8;
    let mut buf = [0u8; 2048];
    let mut last_heartbeat = Instant::now();
    deck_log("io thread started");
    loop {
        if open.is_none() {
            let opened = HidApi::new()
                .ok()
                .and_then(|api| open_akp05e(&api).map(|handle| (api, handle)));
            match opened {
                Some(pair) => {
                    errors = 0;
                    last_heartbeat = Instant::now() - std::time::Duration::from_secs(60);
                    open = Some(pair);
                    deck_log("opened deck, waiting for firmware to settle");
                    // A deck that just re-enumerated is still booting. Pictures sent in
                    // that window are dropped and nothing sends them again, which left the
                    // panel black.
                    std::thread::sleep(std::time::Duration::from_millis(2500));
                    let generation = DECK_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
                    deck_log(&format!("deck ready, generation {generation}"));
                }
                None => {
                    while let Ok(job) = jobs.try_recv() {
                        let _ = job.reply.send(Err("Deck is not open".into()));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    continue;
                }
            }
        }
        let handle = &open.as_ref().expect("open deck").1;
        let mut lost = false;
        // The firmware reboots itself about 45s after the last heartbeat. The official
        // app sends `CRT CONNECT` on a timer. Without it the deck dropped off the bus
        // every 46 seconds, which looked like it connecting and reconnecting.
        if last_heartbeat.elapsed() >= std::time::Duration::from_secs(8) {
            last_heartbeat = Instant::now();
            if debug_flag("no-heartbeat") {
                deck_log("heartbeat skipped (debug flag)");
            } else if let Err(err) = handle.write(&build_heartbeat_packet()) {
                deck_log(&format!("heartbeat write failed: {err}"));
                lost = true;
            } else {
                deck_log("heartbeat sent");
            }
        }
        if debug_flag_take("wake-now") {
            let lig = build_brightness_packet(95);
            let a = handle.write(&build_screen_on_packet());
            let b = handle.write(&lig);
            deck_log(&format!("debug wake-now: DIS={a:?} LIG={b:?}"));
        }
        if debug_flag_take("brightness-0") {
            let r = handle.write(&build_brightness_packet(0));
            deck_log(&format!("debug brightness-0: {r:?}"));
        }
        if let Some(script) = debug_file_take("deck-cmd.txt") {
            run_debug_script(handle, &script);
        }
        while let Ok(job) = jobs.try_recv() {
            let started = Instant::now();
            let mut result = Ok(());
            for (data, pause_ms) in &job.steps {
                if let Err(err) = handle.write(data) {
                    result = Err(format!("USB HID write failed: {err}"));
                    lost = true;
                    break;
                }
                if *pause_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(*pause_ms));
                }
            }
            let first = job
                .steps
                .first()
                .map(|(data, _)| String::from_utf8_lossy(&data[6..9.min(data.len())]).to_string())
                .unwrap_or_default();
            // Skip frame-by-frame GIF writes. Log everything else.
            if job.steps.len() > 1 || result.is_err() {
                deck_log(&format!(
                    "job first={first} packets={} took={}ms result={:?}",
                    job.steps.len(),
                    started.elapsed().as_millis(),
                    result
                ));
            }
            let _ = job.reply.send(result);
            if lost {
                break;
            }
        }
        if !lost {
            match handle.read_timeout(&mut buf, 15) {
                Ok(0) => errors = 0,
                Ok(n) => {
                    errors = 0;
                    let hex = buf[..n.min(14)]
                        .iter()
                        .map(|b| format!("{b:02X}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    deck_log(&format!("report {hex}"));
                    let _ = reports.send(buf[..n].to_vec());
                }
                Err(err) => {
                    errors = errors.saturating_add(1);
                    deck_log(&format!("read error #{errors}: {err}"));
                    lost = errors >= 3;
                }
            }
        }
        if lost {
            deck_log("deck connection lost, closing");
            open = None;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}

/// Queues packets for the deck's single connection and waits for them to be sent.
pub fn deck_write(steps: Vec<(Vec<u8>, u64)>) -> Result<(), String> {
    if steps.is_empty() {
        return Ok(());
    }
    let (reply, wait) = mpsc::channel();
    deck_io()
        .jobs
        .send(DeckJob { steps, reply })
        .map_err(|e| e.to_string())?;
    wait.recv_timeout(std::time::Duration::from_secs(15))
        .map_err(|_| "Deck write timed out".to_string())?
}

/// Next button/knob/touch report read from the deck, if one arrives in time.
pub fn deck_next_report(timeout_ms: u64) -> Option<Vec<u8>> {
    let io = deck_io();
    let rx = io.reports.lock().ok()?;
    rx.recv_timeout(std::time::Duration::from_millis(timeout_ms)).ok()
}

/// Increases each time the deck is opened. A change means the layout must be sent again.
pub fn deck_generation() -> u64 {
    let _ = deck_io();
    DECK_GENERATION.load(Ordering::SeqCst)
}

/// Makes the input worker send the whole layout again, as if the deck had been replugged.
/// Used after another program (the official Stream Dock app) was drawing on the deck.
pub fn request_full_push(reason: &str) {
    let _ = deck_io();
    DECK_GENERATION.fetch_add(1, Ordering::SeqCst);
    deck_log(&format!("full push requested: {reason}"));
}

/// Writes a line to `%APPDATA%\DeckForge\deck-io.log` for events outside the IO thread.
pub fn deck_note(line: &str) {
    deck_log(line);
}

fn open_crt_device(api: &HidApi, dev: &DeviceDescriptor) -> Result<hidapi::HidDevice, String> {
    // Try opening by preferred MI_00 path first, then fallback to open(vid, pid)
    for info in api.device_list() {
        if info.vendor_id() == dev.vid && info.product_id() == dev.pid {
            let p = info.path().to_string_lossy().to_lowercase();
            if p.contains("mi_00") || info.interface_number() == 0 {
                if let Ok(h) = info.open_device(api) {
                    return Ok(h);
                }
            }
        }
    }
    api.open(dev.vid, dev.pid)
        .map_err(|e| format!("Could not open USB HID {}: {}", dev.vid_pid_hex, e))
}

pub fn get_cache_stats(cache: &AntiFreezeFrameCache) -> FrameCacheStats {
    cache.stats.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::frame_cache::AntiFreezeFrameCache;
    use crate::media::MediaStore;

    fn job(index: u8) -> ScreenJob {
        ScreenJob {
            index,
            title: format!("Key {index}"),
            show_title: true,
            icon_name: "circle".into(),
            accent: "#8B5CF6".into(),
            bg: "#151925".into(),
            media_id: None,
            fit: "cover".into(),
            speed: 1.0,
            loop_playback: false,
            level: None,
        }
    }

    fn zone(label: &str) -> ZoneJob {
        ZoneJob {
            label: label.into(),
            value_text: "Live".into(),
            accent: "#22D3EE".into(),
            media_id: None,
            fit: "cover".into(),
            speed: 1.0,
            loop_playback: false,
        }
    }

    fn bat_slots(bursts: &[HidBurst]) -> Vec<u8> {
        bursts
            .iter()
            .filter_map(|burst| {
                let report = burst.reports.first()?;
                if report.len() > 13 && &report[6..9] == b"BAT" {
                    Some(report[13])
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn akp05e_publish_puts_each_row_on_its_own_keys() {
        let dev = builtin_device_catalog()
            .into_iter()
            .find(|d| d.pid == 0x3013)
            .expect("akp05e catalog entry");
        let keys: Vec<ScreenJob> = (0..10).map(job).collect();
        let zones = vec![
            zone("SPOTIFY"),
            zone("CPU LOAD"),
            zone("MIXER"),
            zone("MIC MUTE"),
        ];
        let media = MediaStore::default();
        let mut cache = AntiFreezeFrameCache::new();
        let mut diags = Vec::new();
        let outcome = publish_screens(&dev, &keys, &zones, None, &media, &mut cache, &mut diags);
        assert_eq!(
            bat_slots(&outcome.bursts),
            vec![11, 12, 13, 14, 15, 6, 7, 8, 9, 10, 1, 2, 3, 4]
        );
        assert!(outcome.bursts.iter().all(|burst| {
            burst
                .reports
                .first()
                .map(|report| &report[6..9] != b"DRA")
                .unwrap_or(true)
        }));
    }
}
