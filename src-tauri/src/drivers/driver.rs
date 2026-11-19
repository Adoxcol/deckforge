use crate::drivers::akp05e_protocol::{
    self, HardwareInputEvent, InputControlKind, HID_REPORT_LEN,
};
use crate::drivers::hal::DeviceDescriptor;

/// Protocol abstraction for hardware stream decks so CRT-V4 (Ajazz AKP05E Pro),
/// CRT-V2 (MiraBox N3/293S), and Elgato HID devices share one driver interface.
pub trait DeckDriver: Send + Sync {
    fn protocol_family(&self) -> &'static str;
    fn report_size(&self) -> usize;
    fn matches(&self, vid: u16, pid: u16) -> bool;
    fn build_init_packets(&self) -> Vec<Vec<u8>>;
    fn build_brightness_packet(&self, percent: u8) -> Vec<u8>;
    fn build_key_image_packets(&self, slot_1based: u8, jpeg_bytes: &[u8]) -> Vec<Vec<u8>>;
    fn build_knob_led_packet(&self, _colors: &[[u8; 3]; 4]) -> Option<Vec<u8>> {
        None
    }
    fn build_reset_leds_packet(&self) -> Option<Vec<u8>> {
        None
    }
    fn parse_input(
        &self,
        device_id: &str,
        report: &[u8],
        timestamp_ms: u64,
    ) -> Option<HardwareInputEvent>;
}

/// CRT-V4 (1024-byte payload + 1-byte Report ID = 1025 bytes): Ajazz AKP05E / AKP05E Pro / N4 Pro.
pub struct CrtV4Driver;

impl DeckDriver for CrtV4Driver {
    fn protocol_family(&self) -> &'static str {
        "CRT-V4"
    }

    fn report_size(&self) -> usize {
        HID_REPORT_LEN
    }

    fn matches(&self, vid: u16, pid: u16) -> bool {
        (vid == 0x0300 && matches!(pid, 0x3013 | 0x3004 | 0x3006 | 0x3014))
            || (vid == 0x6603 && pid == 0x1009)
    }

    fn build_init_packets(&self) -> Vec<Vec<u8>> {
        vec![
            akp05e_protocol::build_screen_on_packet().to_vec(),
            akp05e_protocol::build_button_mode_packet().to_vec(),
        ]
    }

    fn build_brightness_packet(&self, percent: u8) -> Vec<u8> {
        akp05e_protocol::build_brightness_packet(percent).to_vec()
    }

    fn build_key_image_packets(&self, slot_1based: u8, jpeg_bytes: &[u8]) -> Vec<Vec<u8>> {
        akp05e_protocol::build_key_image_packets(slot_1based, jpeg_bytes)
            .into_iter()
            .map(|r| r.to_vec())
            .collect()
    }

    fn build_knob_led_packet(&self, colors: &[[u8; 3]; 4]) -> Option<Vec<u8>> {
        Some(akp05e_protocol::build_knob_led_packet(colors).to_vec())
    }

    fn build_reset_leds_packet(&self) -> Option<Vec<u8>> {
        Some(akp05e_protocol::build_reset_leds_packet().to_vec())
    }

    fn parse_input(
        &self,
        device_id: &str,
        report: &[u8],
        timestamp_ms: u64,
    ) -> Option<HardwareInputEvent> {
        akp05e_protocol::parse_input_report(device_id, report, timestamp_ms)
    }
}

/// CRT-V2 (512-byte payload + 1-byte Report ID = 513 bytes): MiraBox N3 / 293S / 293V3.
pub struct CrtV2Driver;

const CRT_V2_PAYLOAD: usize = 512;
const CRT_V2_REPORT: usize = 513;

impl CrtV2Driver {
    fn base_report(cmd: &[u8]) -> Vec<u8> {
        let mut buf = vec![0u8; CRT_V2_REPORT];
        buf[0] = 0x00;
        buf[1..4].copy_from_slice(b"CRT");
        for (i, &b) in cmd.iter().enumerate() {
            if 6 + i < CRT_V2_REPORT {
                buf[6 + i] = b;
            }
        }
        buf
    }
}

impl DeckDriver for CrtV2Driver {
    fn protocol_family(&self) -> &'static str {
        "CRT-V2"
    }

    fn report_size(&self) -> usize {
        CRT_V2_REPORT
    }

    fn matches(&self, vid: u16, pid: u16) -> bool {
        (vid == 0x6603 && pid != 0x1009) || vid == 0x5548
    }

    fn build_init_packets(&self) -> Vec<Vec<u8>> {
        vec![Self::base_report(b"DIS")]
    }

    fn build_brightness_packet(&self, percent: u8) -> Vec<u8> {
        let mut buf = Self::base_report(b"LIG");
        buf[11] = percent.min(100);
        buf
    }

    fn build_key_image_packets(&self, slot_1based: u8, jpeg_bytes: &[u8]) -> Vec<Vec<u8>> {
        let len = jpeg_bytes.len().min(0xFFFF) as u16;
        let mut packets = Vec::with_capacity((jpeg_bytes.len() / CRT_V2_PAYLOAD) + 3);
        let mut header = Self::base_report(b"BAT");
        header[11] = (len >> 8) as u8;
        header[12] = (len & 0xFF) as u8;
        header[13] = slot_1based.max(1);
        packets.push(header);
        for chunk in jpeg_bytes.chunks(CRT_V2_PAYLOAD) {
            let mut pkt = vec![0u8; CRT_V2_REPORT];
            pkt[1..1 + chunk.len()].copy_from_slice(chunk);
            packets.push(pkt);
        }
        packets.push(Self::base_report(b"STP"));
        packets
    }

    fn build_knob_led_packet(&self, colors: &[[u8; 3]; 4]) -> Option<Vec<u8>> {
        let mut buf = Self::base_report(b"SETLB");
        for (k, &[r, g, b]) in colors.iter().enumerate() {
            let base = 11 + k * 3;
            buf[base] = r;
            buf[base + 1] = g;
            buf[base + 2] = b;
        }
        Some(buf)
    }

    fn build_reset_leds_packet(&self) -> Option<Vec<u8>> {
        Some(Self::base_report(b"DELED"))
    }

    fn parse_input(
        &self,
        device_id: &str,
        report: &[u8],
        timestamp_ms: u64,
    ) -> Option<HardwareInputEvent> {
        akp05e_protocol::parse_input_report(device_id, report, timestamp_ms)
    }
}

/// Elgato Stream Deck v2 HID protocol (`VID 0x0FD9`: MK.2, +, XL).
pub struct ElgatoHidDriver;

const ELGATO_IMAGE_REPORT: usize = 1024;
const ELGATO_IMAGE_HEADER: usize = 8;
const ELGATO_IMAGE_CHUNK: usize = ELGATO_IMAGE_REPORT - ELGATO_IMAGE_HEADER;

impl DeckDriver for ElgatoHidDriver {
    fn protocol_family(&self) -> &'static str {
        "Elgato-HID"
    }

    fn report_size(&self) -> usize {
        ELGATO_IMAGE_REPORT
    }

    fn matches(&self, vid: u16, _pid: u16) -> bool {
        vid == 0x0FD9
    }

    fn build_init_packets(&self) -> Vec<Vec<u8>> {
        // Feature report 0x03 0x02 resets the Elgato Stream Deck key display.
        let mut reset = vec![0u8; 32];
        reset[0] = 0x03;
        reset[1] = 0x02;
        vec![reset]
    }

    fn build_brightness_packet(&self, percent: u8) -> Vec<u8> {
        let mut buf = vec![0u8; 32];
        buf[0] = 0x03;
        buf[1] = 0x08;
        buf[2] = percent.min(100);
        buf
    }

    fn build_key_image_packets(&self, slot_1based: u8, jpeg_bytes: &[u8]) -> Vec<Vec<u8>> {
        let key_index = slot_1based.saturating_sub(1);
        let chunks: Vec<&[u8]> = if jpeg_bytes.is_empty() {
            vec![&[]]
        } else {
            jpeg_bytes.chunks(ELGATO_IMAGE_CHUNK).collect()
        };
        let last_idx = chunks.len().saturating_sub(1);
        chunks
            .into_iter()
            .enumerate()
            .map(|(page, chunk)| {
                let mut pkt = vec![0u8; ELGATO_IMAGE_REPORT];
                let len = chunk.len() as u16;
                let page_u16 = page as u16;
                pkt[0] = 0x02;
                pkt[1] = 0x07;
                pkt[2] = key_index;
                pkt[3] = if page == last_idx { 1 } else { 0 };
                pkt[4] = (len & 0xFF) as u8;
                pkt[5] = (len >> 8) as u8;
                pkt[6] = (page_u16 & 0xFF) as u8;
                pkt[7] = (page_u16 >> 8) as u8;
                pkt[8..8 + chunk.len()].copy_from_slice(chunk);
                pkt
            })
            .collect()
    }

    fn parse_input(
        &self,
        device_id: &str,
        report: &[u8],
        timestamp_ms: u64,
    ) -> Option<HardwareInputEvent> {
        // Elgato v2 button report: [0x01, 0x00, count_lo, count_hi, state_0, state_1, ...]
        if report.len() < 5 || report[0] != 0x01 || report[1] != 0x00 {
            return None;
        }
        let count = u16::from_le_bytes([report[2], report[3]]) as usize;
        let states = &report[4..report.len().min(4 + count)];
        let pressed_idx = states.iter().position(|&b| b != 0)?;
        let raw_hex = report[..report.len().min(16)]
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        Some(HardwareInputEvent {
            device_id: device_id.to_string(),
            kind: InputControlKind::LcdKey,
            control_index: pressed_idx as u8,
            state_code: 1,
            pressed: true,
            raw_hex,
            timestamp_ms,
            x: 0,
        })
    }
}

/// Resolves the appropriate `DeckDriver` implementation for a catalog or discovered device.
pub fn driver_for_device(dev: &DeviceDescriptor) -> Box<dyn DeckDriver> {
    let candidates: [Box<dyn DeckDriver>; 3] = [
        Box::new(CrtV4Driver),
        Box::new(CrtV2Driver),
        Box::new(ElgatoHidDriver),
    ];
    for driver in candidates {
        if driver.matches(dev.vid, dev.pid) {
            return driver;
        }
    }
    if dev.protocol_family.starts_with("Elgato") {
        Box::new(ElgatoHidDriver)
    } else if dev.protocol_family.contains("V2") {
        Box::new(CrtV2Driver)
    } else {
        Box::new(CrtV4Driver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::hal::builtin_device_catalog;

    #[test]
    fn resolves_correct_driver_for_each_catalog_family() {
        let catalog = builtin_device_catalog();
        let akp = catalog.iter().find(|d| d.vid == 0x0300 && d.pid == 0x3013).unwrap();
        let drv = driver_for_device(akp);
        assert_eq!(drv.protocol_family(), "CRT-V4");
        assert_eq!(drv.report_size(), 1025);
        assert!(drv.build_knob_led_packet(&[[255, 0, 0]; 4]).is_some());

        if let Some(elgato) = catalog.iter().find(|d| d.vid == 0x0FD9) {
            let edrv = driver_for_device(elgato);
            assert_eq!(edrv.protocol_family(), "Elgato-HID");
            assert_eq!(edrv.report_size(), 1024);
            let bright = edrv.build_brightness_packet(75);
            assert_eq!(&bright[0..3], &[0x03, 0x08, 75]);
            let imgs = edrv.build_key_image_packets(1, &[0xFF, 0xD8, 0xFF, 0xD9]);
            assert_eq!(imgs.len(), 1);
            assert_eq!(&imgs[0][0..5], &[0x02, 0x07, 0x00, 0x01, 0x04]);
            let mut input = vec![0u8; 20];
            input[0] = 0x01;
            input[2] = 15;
            input[4 + 3] = 1;
            let evt = edrv.parse_input(&elgato.id, &input, 42).unwrap();
            assert_eq!(evt.kind, InputControlKind::LcdKey);
            assert_eq!(evt.control_index, 3);
        }

        let v2 = CrtV2Driver;
        let v2_imgs = v2.build_key_image_packets(2, &[0xAA; 600]);
        assert_eq!(v2_imgs.len(), 4); // header + 2 chunks + STP
        assert_eq!(v2_imgs[0].len(), 513);
        assert_eq!(&v2_imgs[0][6..9], b"BAT");
        assert_eq!(&v2_imgs[3][6..9], b"STP");
    }
}
