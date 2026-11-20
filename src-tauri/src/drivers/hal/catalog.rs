use hidapi::HidApi;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

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
