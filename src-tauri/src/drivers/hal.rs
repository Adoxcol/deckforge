use crate::drivers::akp05e_protocol::{
    akp05e_key_image_slot, akp05e_touch_zone_slot, build_brightness_packet, build_clear_key_packet,
    build_button_mode_packet, build_heartbeat_packet, build_knob_led_packet,
    build_led_brightness_packet, build_led_sleep_mode_packet, build_reset_leds_packet,
    build_screen_on_packet, build_touch_strip_packets, encode_procedural_key_jpeg,
    encode_touch_strip_jpeg, parse_hex_color,
};
use crate::drivers::frame_cache::{AntiFreezeFrameCache, FrameCacheStats};
use crate::drivers::driver::driver_for_device;
use super::scheduler::{self, IoStep, Transport};
use crate::media::{pace_delay, MediaStore, RenderedFrames, ZonePaint};
use hidapi::HidApi;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use crate::LockExt;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

pub fn default_knob_rgb(index: usize) -> [u8; 3] {
    match index {
        0 => [0x35, 0xC9, 0x8B],
        1 => [0x56, 0xB9, 0xF2],
        2 => [0x8B, 0x5C, 0xF6],
        _ => [0xF4, 0xB8, 0x60],
    }
}

pub fn parse_knob_rgb_list(colors_hex: &[String]) -> [[u8; 3]; 4] {
    let mut rgb = [[0u8; 3]; 4];
    for (k, slot) in rgb.iter_mut().enumerate() {
        let fallback = default_knob_rgb(k);
        *slot = colors_hex
            .get(k)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|hex| parse_hex_color(hex, fallback))
            .unwrap_or(fallback);
    }
    rgb
}

/// Sends a single `CRT\0\0SETLB` frame directly without appending diagnostic entries (for LED animations).
pub fn write_knob_led_rgb(dev: &DeviceDescriptor, rgb: &[[u8; 3]; 4]) -> Result<(), String> {
    if !dev.connected || dev.is_simulator {
        return Ok(());
    }
    let pkt = build_knob_led_packet(rgb);
    deck_write(vec![(pkt.to_vec(), 0)])
}

/// Sends a `CRT\0\0SETLB` packet setting the 4 rotary knob RGB LEDs on the deck.
pub fn send_hardware_knob_leds(
    dev: &DeviceDescriptor,
    colors_hex: &[String],
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> Result<String, String> {
    let ts = now_ms();
    let mut rgb = [[0u8; 3]; 4];
    for (k, slot) in rgb.iter_mut().enumerate() {
        let fallback = default_knob_rgb(k);
        *slot = colors_hex
            .get(k)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|hex| parse_hex_color(hex, fallback))
            .unwrap_or(fallback);
    }
    let pkt = build_knob_led_packet(&rgb);
    let hex_preview = pkt[1..23]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    if !dev.connected {
        diagnostics.push(DiagnosticEntry {
            id: format!("diag-led-{}", ts),
            timestamp_ms: ts,
            level: "info".to_string(),
            device_id: dev.id.clone(),
            stage: "SIM_KNOB_LED".to_string(),
            message: format!("Simulated knob LED update ({})", dev.model_name),
            packet_hex: Some(hex_preview),
        });
        return Ok(format!("Simulator knob LEDs updated on {}", dev.model_name));
    }

    deck_write(vec![(pkt.to_vec(), 0)])?;

    diagnostics.push(DiagnosticEntry {
        id: format!("diag-led-{}", ts),
        timestamp_ms: ts,
        level: "success".to_string(),
        device_id: dev.id.clone(),
        stage: "CRT_SETLB_LED".to_string(),
        message: format!("Applied knob LED colors to {}", dev.model_name),
        packet_hex: Some(hex_preview),
    });

    Ok(format!("Updated knob LEDs on {}", dev.model_name))
}

/// Sends a `CRT\0\0DELED` packet resetting knob LEDs to factory default.
pub fn reset_hardware_knob_leds(
    dev: &DeviceDescriptor,
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> Result<String, String> {
    let ts = now_ms();
    let pkt = build_reset_leds_packet();
    let hex_preview = pkt[1..11]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    if !dev.connected {
        diagnostics.push(DiagnosticEntry {
            id: format!("diag-led-reset-{}", ts),
            timestamp_ms: ts,
            level: "info".to_string(),
            device_id: dev.id.clone(),
            stage: "SIM_KNOB_LED".to_string(),
            message: format!("Simulated knob LED reset ({})", dev.model_name),
            packet_hex: Some(hex_preview),
        });
        return Ok(format!("Simulator knob LEDs reset on {}", dev.model_name));
    }

    deck_write(vec![(pkt.to_vec(), 0)])?;

    diagnostics.push(DiagnosticEntry {
        id: format!("diag-led-reset-{}", ts),
        timestamp_ms: ts,
        level: "success".to_string(),
        device_id: dev.id.clone(),
        stage: "CRT_DELED_RESET".to_string(),
        message: format!("Reset knob LEDs to factory default on {}", dev.model_name),
        packet_hex: Some(hex_preview),
    });

    Ok(format!("Reset knob LEDs to factory default on {}", dev.model_name))
}

/// Sends a `CRT\0\0LBLIG` packet adjusting LED ring/strip brightness (`0..=100`).
pub fn send_hardware_led_brightness(
    dev: &DeviceDescriptor,
    percent: u8,
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> Result<String, String> {
    let ts = now_ms();
    let clamped = percent.min(100);
    let pkt = build_led_brightness_packet(clamped);
    let hex_preview = pkt[1..12]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    if !dev.connected {
        diagnostics.push(DiagnosticEntry {
            id: format!("diag-lblig-{}", ts),
            timestamp_ms: ts,
            level: "info".to_string(),
            device_id: dev.id.clone(),
            stage: "SIM_LED_BRIGHTNESS".to_string(),
            message: format!("Simulated LED brightness {}% ({})", clamped, dev.model_name),
            packet_hex: Some(hex_preview),
        });
        return Ok(format!("Simulator LED brightness set to {}%", clamped));
    }

    deck_write(vec![(pkt.to_vec(), 0)])?;

    diagnostics.push(DiagnosticEntry {
        id: format!("diag-lblig-{}", ts),
        timestamp_ms: ts,
        level: "success".to_string(),
        device_id: dev.id.clone(),
        stage: "CRT_LBLIG".to_string(),
        message: format!("Set LED brightness to {}% on {}", clamped, dev.model_name),
        packet_hex: Some(hex_preview),
    });

    Ok(format!("LED brightness set to {}% on {}", clamped, dev.model_name))
}

/// Sends a `CRT\0\0QUCMD` packet configuring LED sleep behavior (`0x1F` follow system, `0x11` always on).
pub fn send_hardware_led_sleep_mode(
    dev: &DeviceDescriptor,
    follow_system_sleep: bool,
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> Result<String, String> {
    let ts = now_ms();
    let pkt = build_led_sleep_mode_packet(follow_system_sleep);
    let mode_label = if follow_system_sleep { "Follow PC sleep" } else { "Always on" };
    let hex_preview = pkt[1..12]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    if !dev.connected {
        diagnostics.push(DiagnosticEntry {
            id: format!("diag-qucmd-{}", ts),
            timestamp_ms: ts,
            level: "info".to_string(),
            device_id: dev.id.clone(),
            stage: "SIM_LED_SLEEP".to_string(),
            message: format!("Simulated LED sleep mode: {} ({})", mode_label, dev.model_name),
            packet_hex: Some(hex_preview),
        });
        return Ok(format!("Simulator LED mode set to {}", mode_label));
    }

    deck_write(vec![(pkt.to_vec(), 0)])?;

    diagnostics.push(DiagnosticEntry {
        id: format!("diag-qucmd-{}", ts),
        timestamp_ms: ts,
        level: "success".to_string(),
        device_id: dev.id.clone(),
        stage: "CRT_QUCMD_LED".to_string(),
        message: format!("Set LED sleep mode to {} on {}", mode_label, dev.model_name),
        packet_hex: Some(hex_preview),
    });

    Ok(format!("LED mode set to {} on {}", mode_label, dev.model_name))
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
    if !dev.connected || dev.is_simulator {
        return Err("Deck is not connected".into());
    }
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
fn nudged_brightness(percent: u8) -> u8 {
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

// The deck is opened exactly once, by one thread. That thread reads button
// reports and performs every write that other threads queue. Opening the device
// from several places at once is what made the firmware reset, drop icons, and
// sleep the panel on a press.
struct DeckJob {
    queued_at: Instant,
    steps: Vec<IoStep>,
    reply: mpsc::Sender<Result<(), String>>,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
    connection_epoch: u64,
}

impl DeckJob {
    fn validate(&self, now: Instant, connection_epoch: u64) -> Result<(), String> {
        scheduler::check_deadline(now, self.deadline, &self.cancelled)?;
        if self.connection_epoch != connection_epoch {
            return Err("Deck connection changed; stale output discarded".into());
        }
        Ok(())
    }
}

struct DeckIo {
    jobs: mpsc::SyncSender<DeckJob>,
    reports: Mutex<mpsc::Receiver<Vec<u8>>>,
}

static DECK_IO: OnceLock<DeckIo> = OnceLock::new();
static DECK_GENERATION: AtomicU64 = AtomicU64::new(0);
// Unlike DECK_GENERATION this changes only on a real connection open, not a resend.
static CONNECTION_EPOCH: AtomicU64 = AtomicU64::new(0);
static CONTROL: Mutex<ControlPolicy> = Mutex::new(ControlPolicy { paused: false, vendor_running: false });
static CONTROL_OPEN: AtomicBool = AtomicBool::new(false);

struct ControlPolicy {
    paused: bool,
    vendor_running: bool,
}

impl ControlPolicy {
    fn observe_vendor(&mut self, running: bool) {
        self.vendor_running = running;
        // Once a conflict is seen, only an explicit resume may reclaim the deck.
        if running { self.paused = true; }
    }
    fn available(&self) -> bool { !self.paused && !self.vendor_running }
    fn resume(&mut self) -> Result<(), String> {
        if self.vendor_running {
            return Err("Close the AJAZZ/Stream Dock app and its tray watcher, then choose Use DeckForge. DeckForge will not close them for you.".into());
        }
        self.paused = false;
        Ok(())
    }
    fn suspended_state(&self) -> &'static str {
        if self.vendor_running { "conflict" } else { "paused" }
    }
}

#[cfg(test)]
mod control_tests {
    use super::ControlPolicy;

    #[test]
    fn vendor_conflict_requires_explicit_resume_after_vendor_exits() {
        let mut policy = ControlPolicy { paused: false, vendor_running: false };
        assert!(policy.available());
        policy.observe_vendor(true);
        assert!(!policy.available());
        assert_eq!(policy.suspended_state(), "conflict");
        assert!(policy.resume().is_err());
        policy.observe_vendor(false);
        assert!(!policy.available());
        assert_eq!(policy.suspended_state(), "paused");
        policy.resume().unwrap();
        assert!(policy.available());
    }

    #[test]
    fn manual_pause_survives_vendor_scans() {
        let mut policy = ControlPolicy { paused: true, vendor_running: false };
        policy.observe_vendor(false);
        assert!(!policy.available());
        policy.resume().unwrap();
        assert!(policy.available());
    }
}

pub fn deck_control_available() -> bool { CONTROL.lock_safe().available() }

fn refresh_control_policy(last_scan: &mut Instant) {
    if last_scan.elapsed() >= Duration::from_secs(1) {
        let running = !crate::actions::official_streamdock_processes().is_empty();
        CONTROL.lock_safe().observe_vendor(running);
        *last_scan = Instant::now();
    }
}

pub fn pause_deck_control() -> String {
    CONTROL.lock_safe().paused = true;
    update_status(|s| s.state = if CONTROL_OPEN.load(Ordering::SeqCst) { "pausing" } else { "paused" });
    let _ = deck_io();
    "Pause requested. Wait for DeckForge paused before opening AJAZZ.".into()
}

pub fn resume_deck_control() -> Result<String, String> {
    let running = !crate::actions::official_streamdock_processes().is_empty();
    {
        let mut control = CONTROL.lock_safe();
        control.observe_vendor(running);
        control.resume()?;
    }
    request_full_push("user chose Use DeckForge");
    Ok("DeckForge will reconnect and restore your last applied layout.".into())
}

/// Waits until the I/O thread has closed the deck after a pause, so Windows can restart it.
pub fn wait_deck_released(timeout: Duration) -> bool {
    let until = Instant::now() + timeout;
    while CONTROL_OPEN.load(Ordering::SeqCst) {
        if Instant::now() >= until { return false; }
        std::thread::sleep(Duration::from_millis(50));
    }
    true
}

/// Windows device instance id of the deck's USB device, from its serial number.
/// Only letters and digits are accepted: the id is passed to an elevated command.
pub fn deck_usb_instance_id(serial: &str) -> Result<String, String> {
    let serial = serial.trim();
    if serial.is_empty() || serial.len() > 64 || !serial.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("The deck's serial number is not known, so Windows cannot find it. Plug the deck in and try again.".into());
    }
    Ok(format!("USB\\VID_0300&PID_3013\\{serial}"))
}

/// Exit codes of the elevated `--cycle-deck-port` helper.
pub const CYCLE_OK: u32 = 0;

/// Resets the USB port the deck is plugged into: Windows drops it and enumerates it again,
/// the closest software gets to a replug. Needs administrator rights, so DeckForge starts a
/// second copy of itself elevated (Windows asks the user) that does only this and exits.
#[cfg(target_os = "windows")]
pub fn restart_usb_device(serial: &str) -> Result<(), String> {
    use windows::{
        core::{w, PCWSTR},
        Win32::{
            Foundation::{CloseHandle, WAIT_OBJECT_0},
            System::Threading::{GetExitCodeProcess, WaitForSingleObject},
            UI::{Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW}, WindowsAndMessaging::SW_HIDE},
        },
    };
    deck_usb_instance_id(serial)?; // Only a plain serial reaches the elevated copy.
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let file: Vec<u16> = exe.as_os_str().to_string_lossy().encode_utf16().chain(Some(0)).collect();
    let params: Vec<u16> = format!("--cycle-deck-port {serial}").encode_utf16().chain(Some(0)).collect();
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    if unsafe { ShellExecuteExW(&mut info) }.is_err() {
        return Err("Windows did not allow the restart. It needs you to approve the permission prompt.".into());
    }
    let process = info.hProcess;
    if process.is_invalid() { return Err("Windows did not start the restart.".into()); }
    let waited = unsafe { WaitForSingleObject(process, 30_000) };
    let mut code = 1u32;
    let _ = unsafe { GetExitCodeProcess(process, &mut code) };
    let _ = unsafe { CloseHandle(process) };
    if waited != WAIT_OBJECT_0 { return Err("Windows took too long to restart the deck.".into()); }
    match code {
        CYCLE_OK => Ok(()),
        2 => Err("Windows could not find the deck. Is it plugged in?".into()),
        3 => Err("Windows could not find the USB port the deck is on.".into()),
        _ => Err(format!("Windows could not reset the deck's USB port (code {code}).")),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn restart_usb_device(_serial: &str) -> Result<(), String> {
    Err("Restarting the deck is only available on Windows.".into())
}

/// Runs in the elevated helper copy: finds the deck's hub and port and asks the hub to
/// cycle that port. Returns the process exit code.
#[cfg(target_os = "windows")]
pub fn cycle_deck_port(serial: &str) -> u32 {
    use windows::{
        core::{GUID, PCWSTR},
        Win32::{
            Devices::DeviceAndDriverInstallation::*,
            Foundation::{CloseHandle, GENERIC_WRITE},
            Storage::FileSystem::{CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING},
            System::IO::DeviceIoControl,
        },
    };
    const IOCTL_USB_HUB_CYCLE_PORT: u32 = 0x0022_0444;
    const GUID_DEVINTERFACE_USB_HUB: GUID = GUID::from_u128(0xf18a0e88_c30c_11d0_8815_00a0c906bed8);
    let Ok(id) = deck_usb_instance_id(serial) else { return 2 };
    let id: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let mut dev = 0u32;
        if CM_Locate_DevNodeW(&mut dev, PCWSTR(id.as_ptr()), CM_LOCATE_DEVNODE_NORMAL) != CR_SUCCESS { return 2; }
        let mut hub = 0u32;
        if CM_Get_Parent(&mut hub, dev, 0) != CR_SUCCESS { return 3; }
        let mut port = 0u32;
        let mut len = 4u32;
        if CM_Get_DevNode_Registry_PropertyW(dev, CM_DRP_ADDRESS, None, Some((&mut port as *mut u32).cast()), &mut len, 0) != CR_SUCCESS {
            return 3;
        }
        let mut hub_id = vec![0u16; 512];
        if CM_Get_Device_IDW(hub, &mut hub_id, 0) != CR_SUCCESS { return 3; }
        let mut size = 0u32;
        if CM_Get_Device_Interface_List_SizeW(&mut size, &GUID_DEVINTERFACE_USB_HUB, PCWSTR(hub_id.as_ptr()), CM_GET_DEVICE_INTERFACE_LIST_PRESENT) != CR_SUCCESS || size < 2 {
            return 3;
        }
        let mut list = vec![0u16; size as usize];
        if CM_Get_Device_Interface_ListW(&GUID_DEVINTERFACE_USB_HUB, PCWSTR(hub_id.as_ptr()), &mut list, CM_GET_DEVICE_INTERFACE_LIST_PRESENT) != CR_SUCCESS {
            return 3;
        }
        let Ok(handle) = CreateFileW(PCWSTR(list.as_ptr()), GENERIC_WRITE.0, FILE_SHARE_READ | FILE_SHARE_WRITE, None, OPEN_EXISTING, FILE_FLAGS_AND_ATTRIBUTES(0), None) else {
            return 4;
        };
        // USB_CYCLE_PORT_PARAMS { ConnectionIndex, StatusReturned }
        let mut params = [port, 0u32];
        let mut returned = 0u32;
        let ok = DeviceIoControl(
            handle, IOCTL_USB_HUB_CYCLE_PORT,
            Some(params.as_ptr().cast()), 8, Some(params.as_mut_ptr().cast()), 8,
            Some(&mut returned), None,
        ).is_ok();
        let _ = CloseHandle(handle);
        if ok { CYCLE_OK } else { 5 }
    }
}

/// What the deck connection is doing, for the status line in the app.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckStatus {
    /// `searching` (never found yet), `connected`, or `reconnecting` (it dropped and the
    /// app is waiting for it to come back).
    pub state: &'static str,
    pub opened_at_ms: Option<u64>,
    pub last_push_at_ms: Option<u64>,
    pub last_push_ok: Option<bool>,
    pub last_error: Option<String>,
    /// How many times the deck came back after dropping off USB.
    pub reconnects: u32,
}

static DECK_STATUS: Mutex<DeckStatus> = Mutex::new(DeckStatus {
    state: "searching",
    opened_at_ms: None,
    last_push_at_ms: None,
    last_push_ok: None,
    last_error: None,
    reconnects: 0,
});

fn update_status(change: impl FnOnce(&mut DeckStatus)) {
    let mut status = DECK_STATUS.lock().unwrap_or_else(|e| e.into_inner());
    change(&mut status);
}

/// Records how a send to the deck went. A failed send is always recorded. A successful
/// one only counts when it was a full layout, so frame-by-frame animation writes do not
/// hide an earlier failure.
fn note_push(result: &Result<(), String>, full_layout: bool) {
    match result {
        Ok(()) if full_layout => update_status(|s| {
            s.last_push_at_ms = Some(now_ms());
            s.last_push_ok = Some(true);
            s.last_error = None;
        }),
        Ok(()) => {}
        Err(err) => update_status(|s| {
            s.last_push_at_ms = Some(now_ms());
            s.last_push_ok = Some(false);
            s.last_error = Some(err.clone());
        }),
    }
}

/// Current connection state and the outcome of the last send.
pub fn deck_status() -> DeckStatus {
    let _ = deck_io();
    DECK_STATUS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn deck_io() -> &'static DeckIo {
    DECK_IO.get_or_init(|| {
        let (job_tx, job_rx) = mpsc::sync_channel::<DeckJob>(scheduler::QUEUE_CAPACITY);
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
fn run_debug_script(script: &str) {
    let mut steps = Vec::new();
    for token in script.split_whitespace().take(256) {
        let upper = token.to_ascii_uppercase();
        if let Some(ms) = upper.strip_prefix('W').and_then(|v| v.parse::<u64>().ok()) {
            steps.push(IoStep::command(Vec::new(), ms));
            continue;
        }
        let report = if let Some(percent) = upper.strip_prefix("LIG").and_then(|v| v.parse::<u8>().ok()) {
            build_brightness_packet(percent)
        } else {
            match upper.as_str() {
                "DIS" => build_screen_on_packet(),
                "MOD1" => build_button_mode_packet(),
                "CON" => build_heartbeat_packet(),
                "STP" => crate::drivers::akp05e_protocol::build_finish_packet(),
                "CLE" => build_clear_key_packet(0xFF),
                "VER" => crate::drivers::akp05e_protocol::build_firmware_version_packet(),
                other => { deck_log(&format!("debug unknown token {other}")); continue; }
            }
        };
        steps.push(IoStep::command(report.to_vec(), 150));
    }
    // Debug waits use the same bounded, responsive scheduler as normal traffic.
    if let Err(err) = scheduler::validate_steps(&steps) {
        deck_log(&format!("debug job rejected: {err}"));
        return;
    }
    let (reply, _) = mpsc::channel();
    let job = DeckJob {
        queued_at: Instant::now(),
        steps, reply, deadline: Instant::now() + scheduler::JOB_TIMEOUT,
        cancelled: Arc::new(AtomicBool::new(false)),
        connection_epoch: CONNECTION_EPOCH.load(Ordering::SeqCst),
    };
    if enqueue_job(&deck_io().jobs, job).is_err() { deck_log("debug queue is busy"); }
}

fn validate_report_length(written: usize, expected: usize) -> Result<(), String> {
    if written == expected { Ok(()) }
    else { Err(format!("Short HID write: {written}/{expected} bytes")) }
}

fn write_complete_report(handle: &hidapi::HidDevice, data: &[u8]) -> Result<(), String> {
    let written = handle.write(data).map_err(|e| e.to_string())?;
    validate_report_length(written, data.len())
}

struct HidTransport<'a> {
    handle: &'a hidapi::HidDevice,
    reports: &'a mpsc::Sender<Vec<u8>>,
    heartbeat_enabled: bool,
    last_control_scan: &'a mut Instant,
}

impl Transport for HidTransport<'_> {
    fn now(&self) -> Instant { Instant::now() }
    fn write(&mut self, report: &[u8]) -> Result<(), String> {
        if !deck_control_available() { return Err("DeckForge device control is paused".into()); }
        write_complete_report(self.handle, report)
    }
    fn poll_input(&mut self, timeout_ms: i32) -> Result<(), String> {
        refresh_control_policy(self.last_control_scan);
        if !deck_control_available() { return Err("DeckForge device control is paused".into()); }
        let mut buf = [0u8; 2048];
        let n = self.handle.read_timeout(&mut buf, timeout_ms).map_err(|e| e.to_string())?;
        if n > 0 {
            self.reports.send(buf[..n].to_vec()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn heartbeat(&mut self) -> Result<(), String> {
        if !deck_control_available() { return Err("DeckForge device control is paused".into()); }
        if self.heartbeat_enabled { write_complete_report(self.handle, &build_heartbeat_packet())?; }
        Ok(())
    }
}

fn enqueue_job(jobs: &mpsc::SyncSender<DeckJob>, job: DeckJob) -> Result<(), String> {
    jobs.try_send(job).map_err(|err| match err {
        mpsc::TrySendError::Full(_) => "Deck output queue is busy; try again".into(),
        mpsc::TrySendError::Disconnected(_) => "Deck output worker stopped".into(),
    })
}

fn reject_queued_jobs(jobs: &mpsc::Receiver<DeckJob>, message: &str) {
    // Bound the drain too: new producers cannot keep the I/O loop here forever.
    for _ in 0..scheduler::QUEUE_CAPACITY {
        let Ok(job) = jobs.try_recv() else { break };
        let _ = job.reply.send(Err(message.to_string()));
    }
}

fn deck_io_loop(jobs: mpsc::Receiver<DeckJob>, reports: mpsc::Sender<Vec<u8>>) {
    let mut open: Option<(HidApi, hidapi::HidDevice)> = None;
    let mut errors = 0u8;
    let mut buf = [0u8; 2048];
    let mut last_heartbeat = Instant::now();
    let mut last_full_push: Option<Instant> = None;
    let mut last_control_scan = Instant::now() - Duration::from_secs(2);
    deck_log("io thread started");
    loop {
        refresh_control_policy(&mut last_control_scan);
        if !deck_control_available() {
            open = None; // Drop the HID handle; do not send a disconnect or heartbeat.
            CONTROL_OPEN.store(false, Ordering::SeqCst);
            reject_queued_jobs(&jobs, "DeckForge device control is paused");
            let state = CONTROL.lock_safe().suspended_state();
            update_status(|s| s.state = state);
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        if open.is_none() {
            let opened = HidApi::new()
                .ok()
                .and_then(|api| open_akp05e(&api).map(|handle| (api, handle)));
            match opened {
                Some(pair) => {
                    errors = 0;
                    CONNECTION_EPOCH.fetch_add(1, Ordering::SeqCst);
                    last_full_push = None;
                    last_heartbeat = Instant::now() - std::time::Duration::from_secs(60);
                    open = Some(pair);
                    CONTROL_OPEN.store(true, Ordering::SeqCst);
                    deck_log("opened deck, waiting for firmware to settle");
                    // A deck that just re-enumerated is still booting. Pictures sent in
                    // that window are dropped and nothing sends them again, which left the
                    // panel black.
                    std::thread::sleep(std::time::Duration::from_millis(2500));
                    refresh_control_policy(&mut last_control_scan);
                    if !deck_control_available() { continue; }
                    let generation = DECK_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
                    update_status(|s| {
                        if s.opened_at_ms.is_some() {
                            s.reconnects = s.reconnects.saturating_add(1);
                        }
                        s.state = "connected";
                        s.opened_at_ms = Some(now_ms());
                    });
                    deck_log(&format!("deck ready, generation {generation}"));
                }
                None => {
                    reject_queued_jobs(&jobs, "Deck is not open");
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
            } else if let Err(err) = write_complete_report(handle, &build_heartbeat_packet()) {
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
            run_debug_script(&script);
        }
        // Service one job per outer iteration. The job itself reads input during
        // all pauses and between packets, and checks heartbeat deadlines at safe boundaries.
        if !lost {
            if let Ok(job) = jobs.try_recv() {
                crate::performance::record(crate::performance::Stage::UsbQueue, job.queued_at.elapsed());
                let valid = job.validate(Instant::now(), CONNECTION_EPOCH.load(Ordering::SeqCst));
                if let Err(err) = valid {
                    let _ = job.reply.send(Err(err));
                } else {
                    let full = job.steps.first().is_some_and(|s| s.report.get(6..9) == Some(b"DIS".as_slice()));
                    let settle = if full {
                        last_full_push.map(|done| Duration::from_millis(1500).saturating_sub(done.elapsed())).unwrap_or_default()
                    } else { Duration::ZERO };
                    let started = Instant::now();
                    let mut transport = HidTransport { handle, reports: &reports,
                        heartbeat_enabled: !debug_flag("no-heartbeat"), last_control_scan: &mut last_control_scan };
                    let result = scheduler::run_job(&mut transport, &job.steps, settle,
                        job.deadline, &job.cancelled, &mut last_heartbeat);
                    crate::performance::record(crate::performance::Stage::UsbWrite, started.elapsed());
                    if job.steps.len() > 1 || result.is_err() {
                        deck_log(&format!("job packets={} took={}ms result={result:?}", job.steps.len(), started.elapsed().as_millis()));
                    }
                    if full && result.is_ok() { last_full_push = Some(Instant::now()); }
                    // A timeout or error may leave a partial image on the wire. Close
                    // this session before accepting another job; reconnect restores it.
                    lost = result.is_err();
                    let _ = job.reply.send(result);
                }
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
            update_status(|s| s.state = "reconnecting");
            open = None;
            CONTROL_OPEN.store(false, Ordering::SeqCst);
            reject_queued_jobs(&jobs, "Deck connection lost; queued output discarded");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}

/// Queues packets for the deck's single connection and waits for them to be sent.
pub fn deck_write(steps: Vec<(Vec<u8>, u64)>) -> Result<(), String> {
    deck_write_steps(steps.into_iter().map(|(report, pause)| IoStep::command(report, pause)).collect())
}

fn deck_write_steps(steps: Vec<IoStep>) -> Result<(), String> {
    if steps.is_empty() { return Ok(()); }
    if !deck_control_available() { return Err("DeckForge is paused. Close AJAZZ, then choose Use DeckForge.".into()); }
    scheduler::validate_steps(&steps)?;
    let io = deck_io();
    let (reply, wait) = mpsc::channel();
    let deadline = Instant::now() + scheduler::JOB_TIMEOUT;
    let cancelled = Arc::new(AtomicBool::new(false));
    enqueue_job(&io.jobs, DeckJob {
        queued_at: Instant::now(),
        steps, reply, deadline, cancelled: Arc::clone(&cancelled),
        connection_epoch: CONNECTION_EPOCH.load(Ordering::SeqCst),
    })?;
    match wait.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(result) => result,
        Err(_) => {
            cancelled.store(true, Ordering::Release);
            Err("Deck write timed out or worker stopped".into())
        }
    }
}

/// Next button/knob/touch report read from the deck, if one arrives in time.
pub fn deck_next_report(timeout_ms: u64) -> Option<Vec<u8>> {
    let io = deck_io();
    let rx = io.reports.lock_safe();
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


pub fn get_cache_stats(cache: &AntiFreezeFrameCache) -> FrameCacheStats {
    cache.stats.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued_test_job(epoch: u64) -> (DeckJob, mpsc::Receiver<Result<(), String>>) {
        let (reply, wait) = mpsc::channel();
        (DeckJob { queued_at: Instant::now(), steps: vec![IoStep::command(vec![0], 0)], reply,
            deadline: Instant::now() + scheduler::JOB_TIMEOUT,
            cancelled: Arc::new(AtomicBool::new(false)), connection_epoch: epoch }, wait)
    }

    #[test]
    fn output_queue_rejects_overload_and_disconnect_rejects_pending_work() {
        let (tx, rx) = mpsc::sync_channel(scheduler::QUEUE_CAPACITY);
        let mut replies = Vec::new();
        for _ in 0..scheduler::QUEUE_CAPACITY {
            let (job, reply) = queued_test_job(1);
            enqueue_job(&tx, job).unwrap();
            replies.push(reply);
        }
        assert!(enqueue_job(&tx, queued_test_job(1).0).unwrap_err().contains("busy"));
        reject_queued_jobs(&rx, "connection lost");
        for reply in replies { assert!(reply.try_recv().unwrap().is_err()); }
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn queued_jobs_cannot_cross_connection_epochs_or_outlive_cancellation() {
        let (job, _) = queued_test_job(7);
        assert!(job.validate(Instant::now(), 7).is_ok());
        assert!(job.validate(Instant::now(), 8).is_err());
        assert!(job.validate(job.deadline, 7).is_err());
        job.cancelled.store(true, Ordering::Release);
        assert!(job.validate(Instant::now(), 7).is_err());
    }
    use crate::drivers::frame_cache::AntiFreezeFrameCache;
    use crate::media::MediaStore;

    #[test]
    fn verified_akp_key_burst_preserves_existing_packets() {
        let dev = builtin_device_catalog().remove(0);
        let jpeg = [0xFF, 0xD8, 0x01, 0x02, 0xFF, 0xD9];
        let actual = key_image_burst(&dev, key_image_slot(&dev, 0), &jpeg);
        let expected = crate::drivers::akp05e_protocol::build_key_image_packets(
            key_image_slot(&dev, 0), &jpeg,
        );
        assert_eq!(actual.reports, expected.into_iter().map(|packet| packet.to_vec()).collect::<Vec<_>>());
    }

    #[test]
    fn short_writes_and_disconnected_decks_cannot_report_success() {
        assert!(validate_report_length(1024, 1025).is_err());
        assert!(validate_report_length(0, 1025).is_err());
        assert!(validate_report_length(1025, 1025).is_ok());
        let dev = builtin_device_catalog().remove(0);
        assert!(write_animation_frames(&dev, &[]).is_err());
        assert!(write_one_report(&dev, &[0; 1025]).is_err());
    }

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
            art: None,
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

#[cfg(test)]
mod restart_tests {
    use super::deck_usb_instance_id;

    #[test]
    fn instance_id_only_from_a_plain_serial() {
        assert_eq!(deck_usb_instance_id("4251E8782A0B").unwrap(), r"USB\VID_0300&PID_3013\4251E8782A0B");
        assert!(deck_usb_instance_id("").is_err());
        assert!(deck_usb_instance_id("Not detected").is_err());
        assert!(deck_usb_instance_id("AB\" & calc").is_err());
    }
}
