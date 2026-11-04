use crate::drivers::akp05e_protocol::{
    build_brightness_packet, build_clear_key_packet, build_handshake_packet,
    build_key_image_packets, build_touch_strip_packets, encode_procedural_key_jpeg,
    encode_touch_strip_jpeg,
};
use crate::drivers::frame_cache::{AntiFreezeFrameCache, FrameCacheStats};
use hidapi::HidApi;
use serde::{Deserialize, Serialize};
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

    let api = HidApi::new().map_err(|e| format!("HIDAPI error: {}", e))?;
    let handle = open_crt_device(&api, dev)?;

    // Send handshake first if needed, then brightness
    let han = build_handshake_packet();
    let _ = handle.write(&han);
    handle
        .write(&pkt)
        .map_err(|e| format!("USB HID write failed on {}: {}", dev.vid_pid_hex, e))?;

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
        let han = build_handshake_packet();
        let _ = h.write(&han);
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
            let pkts = build_key_image_packets(idx.saturating_add(1), &jpeg);
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

    // Also render and sync the 800x100 4-zone Touch Strip if the device has a secondary touch screen
    if let Some(ref ts_caps) = dev.touch_strip {
        if !touch_zones.is_empty() {
            let strip_jpeg = encode_touch_strip_jpeg(
                ts_caps.pixel_width,
                ts_caps.pixel_height,
                touch_zones,
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

/// Clears a single key (`0x01..=0x20`) or all keys (`0xFF`) on the device.
pub fn clear_hardware_key(
    dev: &DeviceDescriptor,
    key_idx_1based: u8,
    frame_cache: &mut AntiFreezeFrameCache,
    diagnostics: &mut Vec<DiagnosticEntry>,
) -> Result<String, String> {
    let ts = now_ms();
    frame_cache.invalidate_device(&dev.id);
    let pkt = build_clear_key_packet(key_idx_1based);
    let hex_preview = pkt[1..14]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    if dev.connected {
        if let Ok(api) = HidApi::new() {
            if let Ok(handle) = open_crt_device(&api, dev) {
                let _ = handle.write(&build_handshake_packet());
                let _ = handle.write(&pkt);
            }
        }
    }

    diagnostics.push(DiagnosticEntry {
        id: format!("diag-{}", ts),
        timestamp_ms: ts,
        level: "info".to_string(),
        device_id: dev.id.clone(),
        stage: "CRT_CLE_KEY".to_string(),
        message: format!("Cleared key slot 0x{:02X} on {}", key_idx_1based, dev.model_name),
        packet_hex: Some(hex_preview),
    });

    Ok(format!("Cleared key 0x{:02X}", key_idx_1based))
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
