use image::{ImageBuffer, Rgb, RgbImage};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

/// Windows HID writes on Ajazz AKP05E Pro (`0300:3013`, V4 protocol) prepend a 1-byte Report ID (`0x00`)
/// to a 1024-byte (`0x400`) payload = 1025 bytes (`0x401`) total (`this->outputReportLen = 1024` in `SDLibrary1.dll`).
/// Sending 65-byte reports on a 1025-byte HID endpoint causes Windows `WriteFile` to zero-pad 960 bytes per chunk,
/// corrupting the JPEG stream and turning the screens black.
pub const HID_REPORT_LEN: usize = 1025;
pub const HID_PAYLOAD_LEN: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InputControlKind {
    LcdKey,
    RotaryKnobTurnCw,
    RotaryKnobTurnCcw,
    RotaryKnobPush,
    TouchStripTap,
    TouchStripSwipe,
    /// A finger position on the strip. The input worker turns a short one into a tap
    /// and a long horizontal move into a page change.
    TouchStripPoint,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInputEvent {
    pub device_id: String,
    pub kind: InputControlKind,
    pub control_index: u8,
    pub state_code: u8,
    pub pressed: bool,
    pub raw_hex: String,
    pub timestamp_ms: u64,
    /// Horizontal position on the 800-wide strip. Zero when the report is not a finger point.
    #[serde(default)]
    pub x: u16,
}

/// Builds a 1025-byte Windows HID output report with Report ID `0x00` at byte 0
/// and `"CRT\0\0"` (`43 52 54 00 00`) at payload bytes `0..5` (buffer bytes `1..6`).
fn crt_base_report(cmd: &[u8]) -> [u8; HID_REPORT_LEN] {
    let mut buf = [0u8; HID_REPORT_LEN];
    buf[0] = 0x00; // Report ID 0
    buf[1] = b'C';
    buf[2] = b'R';
    buf[3] = b'T';
    buf[4] = 0x00;
    buf[5] = 0x00;
    for (i, &b) in cmd.iter().enumerate() {
        if 6 + i < HID_REPORT_LEN {
            buf[6 + i] = b;
        }
    }
    buf
}

/// `"CRT\0\0HAN"`. Mirajazz names this `sleep()`: the panel stays dark until the
/// next physical press wakes it. That is the click-to-black, click-again-to-light
/// behavior. Refreshing pictures must not send it.
pub fn build_handshake_packet() -> [u8; HID_REPORT_LEN] {
    let mut buf = crt_base_report(b"HAN");
    buf[11] = 0x01;
    buf
}

/// `"CRT\0\0DIS"`. The N4 / AKP05 protocol uses this to turn the screen on.
/// Mirajazz sends it at the start of `initialize()`, before brightness.
pub fn build_screen_on_packet() -> [u8; HID_REPORT_LEN] {
    crt_base_report(b"DIS")
}

/// `SDLibrary1.dll` @ `0x180023440` (`sendGetHardwareFirmwareVersion`): `"CRT\0\0VER"`
pub fn build_firmware_version_packet() -> [u8; HID_REPORT_LEN] {
    crt_base_report(b"VER")
}

/// `SDLibrary1.dll` @ `0x180023730` (`sendHeartbeatPack`): `"CRT\0\0CONNECT"`
pub fn build_heartbeat_packet() -> [u8; HID_REPORT_LEN] {
    crt_base_report(b"CONNECT")
}

/// `SDLibrary1.dll` @ `0x1800225d0` (`sendAdjustBrightnessPack`): `"CRT\0\0LIG\0\0<brightness>"`
/// Brightness is clamped to `0..=100` (`0x00..=0x64`) and placed at buffer offset `0x0B` (payload byte 10).
pub fn build_brightness_packet(percent: u8) -> [u8; HID_REPORT_LEN] {
    let mut buf = crt_base_report(b"LIG");
    buf[11] = percent.min(100);
    buf
}

/// `"CRT\0\0MOD"` with ASCII `1` at payload byte 10.
/// Button mode is what keeps the ten key images on the panel. The strip still
/// reports taps (`0x40`–`0x43`) and left/right swipes (`0x38`/`0x39`).
/// `SDLibrary1.dll` `addModeChangedCommand` takes this mode byte.
pub fn build_button_mode_packet() -> [u8; HID_REPORT_LEN] {
    let mut buf = crt_base_report(b"MOD");
    buf[11] = b'1';
    buf
}

/// `"CRT\0\0MOD"` with ASCII `2`. The official app calls this Touchbar Mode.
/// On the AKP05E it replaces the key grid: the first press clears those images
/// and the panel stays black until the host redraws it. DeckForge does not send this.
pub fn build_touchbar_mode_packet() -> [u8; HID_REPORT_LEN] {
    let mut buf = crt_base_report(b"MOD");
    buf[11] = b'2';
    buf
}

/// `SDLibrary1.dll` @ `0x18001cfb0` / `0x18001d0f0` (`getClearSingleButtonCommand` / `getClearAllKeysCommand`):
/// `"CRT\0\0CLE\0\0\0<keyIdx>"` (`0xFF` = all keys, `0x01..=0x0A` = single LCD key).
pub fn build_clear_key_packet(key_idx: u8) -> [u8; HID_REPORT_LEN] {
    let mut buf = crt_base_report(b"CLE");
    buf[11] = 0x00;
    buf[12] = key_idx;
    buf
}

/// `SDLibrary1.dll` @ `0x18001d340` (`getFinishCommand`): `"CRT\0\0STP"`
pub fn build_finish_packet() -> [u8; HID_REPORT_LEN] {
    crt_base_report(b"STP")
}

/// `SDLibrary1.dll` @ `0x180022dc0` (`sendDisconnectCommand`): `"CRT\0\0DIS"`
pub fn build_disconnect_packet() -> [u8; HID_REPORT_LEN] {
    crt_base_report(b"DIS")
}

/// `Stream Dock AJAZZ.exe` @ `0x140194550` (`SDLightKnoLabel::sendDeviceLightChange`):
/// `"CRT\0\0SETLB"` (`43 52 54 00 00 53 45 54 4C 42`) followed by 4 RGB triples (`R, G, B`)
/// at payload bytes `10..22` (report buffer bytes `11..23`: `11 + 3 * k .. 13 + 3 * k`).
pub fn build_knob_led_packet(colors: &[[u8; 3]; 4]) -> [u8; HID_REPORT_LEN] {
    let mut buf = crt_base_report(b"SETLB");
    for (k, &[r, g, b]) in colors.iter().enumerate() {
        let base = 11 + k * 3;
        buf[base] = r;
        buf[base + 1] = g;
        buf[base + 2] = b;
    }
    buf
}

/// `Stream Dock AJAZZ.exe` @ `0x140014c60` (`LightSpripColor::sendResetDeviceColor`):
/// `"CRT\0\0DELED"` resets knob/strip LEDs to factory default colors.
pub fn build_reset_leds_packet() -> [u8; HID_REPORT_LEN] {
    crt_base_report(b"DELED")
}

/// AKP05E visual key (row-major, 0 = top left) → 1-based BAT image slot.
///
/// The face is one 800×480 panel, not ten independent screens plus a strip at y=0.
/// Slots 1–4 are the four 176×112 touch windows. The top key row is 11–15.
/// The bottom key row is 6–10. Sending the top row as 1–5 paints those icons
/// onto the touch strip, and an 800×100 image at y=0 covers the real top keys.
pub fn akp05e_key_image_slot(visual_index: u8) -> u8 {
    match visual_index {
        0..=4 => 11 + visual_index,
        5..=9 => visual_index + 1,
        other => other.saturating_add(1).max(1),
    }
}

/// Touch-strip zone (0 = left) → 1-based BAT slot. The four windows are slots 1–4.
pub fn akp05e_touch_zone_slot(zone_index: u8) -> u8 {
    zone_index.saturating_add(1).min(4).max(1)
}

/// `SDLibrary1.dll` @ `0x180023e60` (`sendPicSizeCommand`):
/// Header `"CRT\0\0BAT\0\0<size_hi><size_lo><key_idx_1based>"` followed by 1024-byte JPEG chunks and `"CRT\0\0STP"`.
pub fn build_key_image_packets(key_idx_1based: u8, jpeg_bytes: &[u8]) -> Vec<[u8; HID_REPORT_LEN]> {
    let mut packets = Vec::with_capacity((jpeg_bytes.len() / HID_PAYLOAD_LEN) + 3);
    let len = jpeg_bytes.len().min(0xFFFF) as u16;

    let mut header = crt_base_report(b"BAT");
    header[11] = (len >> 8) as u8;
    header[12] = (len & 0xFF) as u8;
    header[13] = key_idx_1based.max(1);
    packets.push(header);

    for chunk in jpeg_bytes.chunks(HID_PAYLOAD_LEN) {
        let mut pkt = [0u8; HID_REPORT_LEN];
        pkt[0] = 0x00; // Report ID 0
        pkt[1..1 + chunk.len()].copy_from_slice(chunk);
        packets.push(pkt);
    }

    packets.push(build_finish_packet());
    packets
}

/// `SDLibrary1.dll` @ `0x180024620` (`sendSecondaryScreenPicInfo`):
/// Header `"CRT\0\0DRA"` + 24-bit byte length (`[10..13]`), layer (`[13]`), `x` (`[14..16]`), `y` (`[16..18]`),
/// `width` (`[18..20]`, e.g. `800`), `height` (`[20..22]`, e.g. `100`) followed by 1024-byte JPEG chunks and `"CRT\0\0STP"`.
pub fn build_touch_strip_packets(
    jpeg_bytes: &[u8],
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) -> Vec<[u8; HID_REPORT_LEN]> {
    let mut packets = Vec::with_capacity((jpeg_bytes.len() / HID_PAYLOAD_LEN) + 3);
    let len = jpeg_bytes.len() as u32;

    let mut header = crt_base_report(b"DRA");
    // Exact byte layout from 0x180024764..0x18002492f in SDLibrary1.dll:
    header[10] = ((len >> 16) & 0xFF) as u8;
    header[11] = ((len >> 8) & 0xFF) as u8;
    header[12] = (len & 0xFF) as u8;
    header[13] = 0x00; // Layer / screen id
    header[14] = (x >> 8) as u8;
    header[15] = (x & 0xFF) as u8;
    header[16] = (y >> 8) as u8;
    header[17] = (y & 0xFF) as u8;
    header[18] = (width >> 8) as u8;
    header[19] = (width & 0xFF) as u8;
    header[20] = (height >> 8) as u8;
    header[21] = (height & 0xFF) as u8;
    packets.push(header);

    for chunk in jpeg_bytes.chunks(HID_PAYLOAD_LEN) {
        let mut pkt = [0u8; HID_REPORT_LEN];
        pkt[0] = 0x00;
        pkt[1..1 + chunk.len()].copy_from_slice(chunk);
        packets.push(pkt);
    }

    packets.push(build_finish_packet());
    packets
}

/// AKP05E control byte (input report byte 9) → which key, knob, or touch window moved.
/// Key codes 1–10 are the ten LCD keys in visual order. They are not the image-slot numbers.
fn decode_akp05_input(code: u8, state: u8) -> Option<(InputControlKind, u8, bool)> {
    let pressed = state != 0;
    let decoded = match code {
        0x01..=0x0A => (InputControlKind::LcdKey, code - 1, pressed),
        0xA0 => (InputControlKind::RotaryKnobTurnCcw, 0, true),
        0xA1 => (InputControlKind::RotaryKnobTurnCw, 0, true),
        0x50 => (InputControlKind::RotaryKnobTurnCcw, 1, true),
        0x51 => (InputControlKind::RotaryKnobTurnCw, 1, true),
        0x90 => (InputControlKind::RotaryKnobTurnCcw, 2, true),
        0x91 => (InputControlKind::RotaryKnobTurnCw, 2, true),
        0x70 => (InputControlKind::RotaryKnobTurnCcw, 3, true),
        0x71 => (InputControlKind::RotaryKnobTurnCw, 3, true),
        // The strip and the knob clicks are one-shot. The state byte is often 0,
        // and a following release must not toggle play/pause a second time.
        0x40 => (InputControlKind::TouchStripTap, 0, true),
        0x41 => (InputControlKind::TouchStripTap, 1, true),
        0x42 => (InputControlKind::TouchStripTap, 2, true),
        0x43 => (InputControlKind::TouchStripTap, 3, true),
        0x38 => (InputControlKind::TouchStripSwipe, 0, true),
        0x39 => (InputControlKind::TouchStripSwipe, 1, true),
        0x37 => (InputControlKind::RotaryKnobPush, 0, true),
        0x35 => (InputControlKind::RotaryKnobPush, 1, true),
        0x33 => (InputControlKind::RotaryKnobPush, 2, true),
        0x36 => (InputControlKind::RotaryKnobPush, 3, true),
        0x00 => return None,
        other => (InputControlKind::Unknown, other, pressed),
    };
    Some(decoded)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripGestureResult {
    Ignore,
    Tap(u8),
    SwipeNext,
    SwipePrev,
}

#[derive(Debug, Clone, Default)]
pub struct StripGesture {
    start_x: Option<u16>,
    last_x: u16,
}

fn zone_for_strip_x(x: u16) -> u8 {
    ((u32::from(x.min(799)) * 4) / 800).min(3) as u8
}

/// A finger-down starts a gesture. A short lift is a tap. A move of 80px or more
/// changes the page: right goes forward, left goes back.
pub fn observe_strip_point(gesture: &mut StripGesture, phase: u8, x: u16) -> StripGestureResult {
    if x == 0 && phase != 0 {
        return StripGestureResult::Ignore;
    }
    match phase {
        1 => {
            gesture.start_x = Some(x);
            gesture.last_x = x;
            StripGestureResult::Ignore
        }
        2 => {
            if x == 0 {
                return StripGestureResult::Ignore;
            }
            if gesture.start_x.is_none() {
                gesture.start_x = Some(x);
            }
            gesture.last_x = x;
            StripGestureResult::Ignore
        }
        _ => {
            let start = gesture.start_x.take().unwrap_or(x);
            let end = if x == 0 { gesture.last_x } else { x };
            gesture.last_x = end;
            if start == 0 {
                return StripGestureResult::Ignore;
            }
            let delta = i32::from(end) - i32::from(start);
            if delta.abs() >= 80 {
                if delta > 0 {
                    StripGestureResult::SwipeNext
                } else {
                    StripGestureResult::SwipePrev
                }
            } else {
                StripGestureResult::Tap(zone_for_strip_x(start))
            }
        }
    }
}

pub fn resolve_strip_point(gesture: &mut StripGesture, point: &HardwareInputEvent) -> Option<HardwareInputEvent> {
    let result = observe_strip_point(gesture, point.state_code, point.x);
    let (kind, index) = match result {
        StripGestureResult::Ignore => return None,
        StripGestureResult::Tap(zone) => (InputControlKind::TouchStripTap, zone),
        StripGestureResult::SwipeNext => (InputControlKind::TouchStripSwipe, 1),
        StripGestureResult::SwipePrev => (InputControlKind::TouchStripSwipe, 0),
    };
    Some(HardwareInputEvent {
        kind,
        control_index: index,
        state_code: 1,
        pressed: true,
        ..point.clone()
    })
}

fn raw_hex_prefix(payload: &[u8]) -> String {
    payload
        .iter()
        .take(16)
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn point_event(device_id: &str, payload: &[u8], timestamp_ms: u64, phase: u8, x: u16) -> HardwareInputEvent {
    HardwareInputEvent {
        device_id: device_id.to_string(),
        kind: InputControlKind::TouchStripPoint,
        control_index: 0,
        state_code: phase,
        pressed: phase != 0,
        raw_hex: raw_hex_prefix(payload),
        timestamp_ms,
        x,
    }
}

/// Touchbar mode reports the finger. `ACK\0\0OK` is down or up. `ARX` is a move.
/// A command ack has the same OK header and an X of 0, so it stays ignored.
fn touch_strip_point(device_id: &str, payload: &[u8], timestamp_ms: u64) -> Option<HardwareInputEvent> {
    if payload.len() >= 12 && &payload[5..8] == b"ARX" {
        let x = u16::from_be_bytes([payload[10], payload[11]]);
        if x == 0 {
            return None;
        }
        return Some(point_event(device_id, payload, timestamp_ms, 2, x));
    }
    if payload.len() < 13 || &payload[5..7] != b"OK" {
        return None;
    }
    let x = u16::from_be_bytes([payload[11], payload[12]]);
    if x == 0 {
        return None;
    }
    Some(point_event(device_id, payload, timestamp_ms, payload[10], x))
}

fn find_known_control(payload: &[u8]) -> Option<(u8, u8)> {
    let end = payload.len().min(24);
    for i in 0..end {
        let code = payload[i];
        let known = matches!(
            code,
            0x33 | 0x35 | 0x36 | 0x37 | 0x38 | 0x39 | 0x40..=0x43 | 0x50 | 0x51 | 0x70 | 0x71 | 0x90 | 0x91 | 0xA0 | 0xA1
        );
        if known {
            let state = payload.get(i + 1).copied().unwrap_or(1);
            return Some((code, state));
        }
    }
    None
}

/// `SDLibrary1.dll` @ `0x180021280` / `0x180021ce5` (`readDataFromHidDevice`):
/// Parses an input report. Command acks are `"ACK\0\0OK"`. Presses are `"ACK"` plus the control code at byte 9.
pub fn parse_input_report(device_id: &str, report: &[u8], timestamp_ms: u64) -> Option<HardwareInputEvent> {
    let payload = if !report.is_empty() && report[0] == 0x00 {
        &report[1..]
    } else {
        report
    };

    if payload.len() < 11 {
        return None;
    }

    // Button reports start with "ACK". A finished command is "ACK\0\0OK" with no control code.
    // A touchbar-mode tap uses that same header: byte 10 is down/up and bytes 11–12 are the X.
    let (control_id, state_code) = if &payload[0..3] == b"ACK" {
        if payload.len() >= 12 && &payload[5..8] == b"ARX" {
            return touch_strip_point(device_id, payload, timestamp_ms);
        }
        let control_id = payload[9];
        let state_code = payload[10];
        if control_id == 0 && payload.len() >= 7 && &payload[5..7] == b"OK" {
            return touch_strip_point(device_id, payload, timestamp_ms);
        }
        (control_id, state_code)
    } else if let Some(found) = find_known_control(payload) {
        found
    } else {
        return None;
    };

    let (kind, control_index, pressed) = decode_akp05_input(control_id, state_code)?;

    let raw_hex = payload[0..16]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    Some(HardwareInputEvent {
        device_id: device_id.to_string(),
        kind,
        control_index,
        state_code,
        pressed,
        raw_hex,
        timestamp_ms,
        x: 0,
    })
}

/// Parses a `#RRGGBB` hex color string into `[u8; 3]`.
pub(crate) fn parse_hex_color(hex: &str, default: [u8; 3]) -> [u8; 3] {
    let clean = hex.trim_start_matches('#');
    if clean.len() >= 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(default[0]);
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(default[1]);
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(default[2]);
        [r, g, b]
    } else {
        default
    }
}

/// 5x7 bitmap font glyphs for uppercase/lowercase ASCII & symbols so hardware keycaps and touch strip zones
/// display crisp, legible titles matching the DeckForge UI.
fn glyph_5x7(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b10010, 0b10010, 0b10010, 0b11111, 0b00010, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
        '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        '%' => [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        '>' => [0b10000, 0b01000, 0b00100, 0b00010, 0b00100, 0b01000, 0b10000],
        _ => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
    }
}

pub(crate) fn draw_text_centered(
    img: &mut RgbImage,
    text: &str,
    center_x: i32,
    top_y: i32,
    scale: i32,
    color: Rgb<u8>,
) {
    let chars: Vec<char> = text.chars().take(13).collect();
    if chars.is_empty() {
        return;
    }
    let char_advance = 6 * scale;
    let total_w = (chars.len() as i32) * char_advance - scale;
    let start_x = center_x - (total_w / 2);

    for (idx, ch) in chars.into_iter().enumerate() {
        let rows = glyph_5x7(ch);
        let gx = start_x + (idx as i32) * char_advance;
        for (ry, row_bits) in rows.iter().enumerate() {
            for rx in 0..5 {
                if (row_bits >> (4 - rx)) & 1 == 1 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let px = gx + (rx * scale) + sx;
                            let py = top_y + ((ry as i32) * scale) + sy;
                            if px >= 0
                                && py >= 0
                                && (px as u32) < img.width()
                                && (py as u32) < img.height()
                            {
                                img.put_pixel(px as u32, py as u32, color);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// True when `(px, py)` lies within `r` of the segment from `(ax, ay)` to `(bx, by)`.
fn glyph_seg(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32, r: f32) -> bool {
    let (vx, vy) = (bx - ax, by - ay);
    let len2 = vx * vx + vy * vy;
    let t = if len2 <= f32::EPSILON {
        0.0
    } else {
        (((px - ax) * vx + (py - ay) * vy) / len2).clamp(0.0, 1.0)
    };
    let (cx, cy) = (ax + t * vx, ay + t * vy);
    (px - cx).powi(2) + (py - cy).powi(2) <= r * r
}

/// Outline of the axis-aligned rectangle `[x0, x1] x [y0, y1]` with stroke width `t`.
fn glyph_rect(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32, t: f32) -> bool {
    let outer = px >= x0 - t && px <= x1 + t && py >= y0 - t && py <= y1 + t;
    let inner = px > x0 + t && px < x1 - t && py > y0 + t && py < y1 - t;
    outer && !inner
}

/// Arrow pointing along +x. The other three directions rotate the sample point first.
fn glyph_arrow(x: f32, y: f32) -> bool {
    glyph_seg(x, y, -10.0, 0.0, 10.0, 0.0, 1.7)
        || glyph_seg(x, y, 10.0, 0.0, 3.0, -7.0, 1.7)
        || glyph_seg(x, y, 10.0, 0.0, 3.0, 7.0, 1.7)
}

/// A "turn back" arrow pointing left (undo). Mirror `x` for redo.
fn glyph_turn_arrow(x: f32, y: f32) -> bool {
    let head = glyph_seg(x, y, -10.0, -3.0, -4.0, -9.0, 1.6) || glyph_seg(x, y, -10.0, -3.0, -4.0, 3.0, 1.6);
    let top = glyph_seg(x, y, -10.0, -3.0, 2.0, -3.0, 1.6);
    let bend = ((x - 2.0).powi(2) + (y - 3.0).powi(2)).sqrt();
    let curve = (bend - 6.0).abs() < 1.6 && x >= 2.0;
    let bottom = glyph_seg(x, y, 2.0, 9.0, -5.0, 9.0, 1.6);
    head || top || curve || bottom
}

/// Evaluates whether local icon coordinates `(dx, dy)` inside `[-18.0..18.0]` belong to the stroke of `icon_name`.
fn icon_stroke_hit(icon_name: &str, dx: f32, dy: f32) -> bool {
    let dist = (dx * dx + dy * dy).sqrt();
    match icon_name {
        "Radio" => {
            // Center dot + two concentric side wave arcs ((o))
            let center_dot = dist < 3.4;
            let inner_arc = (dist - 7.5).abs() < 1.6 && dx.abs() > 3.2 && dy.abs() < 6.8;
            let outer_arc = (dist - 12.2).abs() < 1.6 && dx.abs() > 5.2 && dy.abs() < 10.5;
            center_dot || inner_arc || outer_arc
        }
        "Clapperboard" => {
            // Movie clapperboard: outer box + top clap bar + diagonal stripes
            let box_outline = dx.abs() <= 12.0
                && dy >= -9.0
                && dy <= 11.0
                && (dx.abs() >= 10.2 || (dy - 11.0).abs() < 1.8 || (dy - (-9.0)).abs() < 1.8 || (dy - (-3.0)).abs() < 1.6);
            let clap_stripes = dy >= -9.0 && dy <= -3.0 && dx.abs() <= 11.0 && (((dx + dy + 24.0) as i32).rem_euclid(6) < 2);
            let play_tri = dy >= 1.0 && dy <= 7.0 && dx >= -3.0 && dx <= 4.0 && (dx + 3.0) <= (4.5 - (dy - 4.0).abs()) * 1.6;
            box_outline || clap_stripes || play_tri
        }
        "Film" => {
            // Filmstrip frame with sprocket holes on left/right
            let outer = dx.abs() <= 11.5
                && dy.abs() <= 11.0
                && (dx.abs() >= 9.8 || dy.abs() >= 9.3 || (dx.abs() - 5.5).abs() < 1.4 || dy.abs() < 1.0);
            let sprockets = dx.abs() >= 6.5 && dx.abs() <= 9.5 && ((dy.abs() - 4.5).abs() < 1.2);
            outer || sprockets
        }
        "MicOff" => {
            // Microphone capsule + stand + diagonal mute slash
            let capsule = dx.abs() <= 4.5 && dy >= -11.0 && dy <= 1.0 && (dx.abs() >= 2.8 || (dy + 11.0).abs() < 1.8 || (dy - 1.0).abs() < 1.8);
            let u_cup = dy >= -2.0 && dy <= 5.5 && (dist - 7.8).abs() < 1.6;
            let stand = (dx.abs() < 1.2 && dy >= 5.5 && dy <= 10.5) || (dy >= 9.5 && dy <= 11.2 && dx.abs() <= 5.5);
            let slash = (dx - dy).abs() < 1.5 && dx.abs() <= 12.5 && dy.abs() <= 12.5;
            capsule || u_cup || stand || slash
        }
        "Volume2" => {
            // Speaker cone + sound waves
            let cone_back = dx >= -11.0 && dx <= -6.0 && dy.abs() <= 4.0 && (dx <= -9.3 || dy.abs() >= 2.4);
            let cone_flare = dx >= -6.0 && dx <= 0.0 && dy.abs() <= (4.0 + (dx + 6.0) * 0.9) && ((dy.abs() - (4.0 + (dx + 6.0) * 0.9)).abs() < 1.8 || (dx - 0.0).abs() < 1.6);
            let wave1 = dx > 2.5 && (dist - 6.8).abs() < 1.5 && dy.abs() < 5.5;
            let wave2 = dx > 4.5 && (dist - 11.2).abs() < 1.5 && dy.abs() < 9.2;
            cone_back || cone_flare || wave1 || wave2
        }
        "Disc" | "CircleDot" => {
            // Record disc: outer ring + inner core dot
            (dist - 11.0).abs() < 1.8 || dist < 3.6
        }
        "ArrowRight" => glyph_arrow(dx, dy),
        "ArrowLeft" => glyph_arrow(-dx, dy),
        "ArrowUp" => glyph_arrow(-dy, dx),
        "ArrowDown" => glyph_arrow(dy, dx),
        "X" => glyph_seg(dx, dy, -8.0, -8.0, 8.0, 8.0, 1.8) || glyph_seg(dx, dy, -8.0, 8.0, 8.0, -8.0, 1.8),
        "Plus" => glyph_seg(dx, dy, -9.0, 0.0, 9.0, 0.0, 1.8) || glyph_seg(dx, dy, 0.0, -9.0, 0.0, 9.0, 1.8),
        "Copy" => {
            // Front sheet outline + the corner of the sheet behind it
            let front = glyph_rect(dx, dy, -10.0, -3.0, 3.0, 10.0, 1.6);
            let back = glyph_seg(dx, dy, -3.0, -3.0, -3.0, -9.0, 1.5)
                || glyph_seg(dx, dy, -3.0, -9.0, 9.0, -9.0, 1.5)
                || glyph_seg(dx, dy, 9.0, -9.0, 9.0, 4.0, 1.5)
                || glyph_seg(dx, dy, 9.0, 4.0, 3.0, 4.0, 1.5);
            front || back
        }
        "Clipboard" => {
            // Board with a clip on top and two text lines
            let board = glyph_rect(dx, dy, -9.0, -8.0, 9.0, 11.0, 1.6);
            let clip = dx.abs() <= 4.5 && (-11.5..=-5.5).contains(&dy);
            let lines = glyph_seg(dx, dy, -5.0, 0.0, 5.0, 0.0, 1.3) || glyph_seg(dx, dy, -5.0, 5.0, 5.0, 5.0, 1.3);
            board || clip || lines
        }
        "Undo" => glyph_turn_arrow(dx, dy),
        "Redo" => glyph_turn_arrow(-dx, dy),
        "Save" => {
            // Floppy disk: outline, shutter at the top, label at the bottom
            let body = glyph_rect(dx, dy, -10.0, -10.0, 10.0, 10.0, 1.6);
            let shutter = dx.abs() <= 5.0 && (-10.0..=-4.0).contains(&dy);
            let label = glyph_rect(dx, dy, -6.0, 1.0, 6.0, 10.0, 1.5);
            body || shutter || label
        }
        "Smile" => {
            let face = (dist - 11.0).abs() < 1.6;
            let eyes = ((dx.abs() - 4.0).powi(2) + (dy + 3.5).powi(2)).sqrt() < 1.9;
            let mouth = (dist - 6.5).abs() < 1.5 && dy >= 2.5;
            face || eyes || mouth
        }
        "Settings" => {
            // Gear: ring, hub, eight teeth
            let angle = dy.atan2(dx) / (std::f32::consts::PI / 4.0);
            let off_tooth = (angle - angle.round()).abs();
            let teeth = (8.4..=11.8).contains(&dist) && off_tooth < 0.2;
            (dist - 7.5).abs() < 1.7 || (dist - 2.6).abs() < 1.3 || teeth
        }
        "Folder" => {
            let body = glyph_rect(dx, dy, -11.0, -6.0, 11.0, 10.0, 1.6);
            let tab = glyph_seg(dx, dy, -11.0, -6.0, -11.0, -10.0, 1.5)
                || glyph_seg(dx, dy, -11.0, -10.0, -3.0, -10.0, 1.5)
                || glyph_seg(dx, dy, -3.0, -10.0, -1.0, -6.0, 1.5);
            body || tab
        }
        "Gamepad2" => {
            let body = glyph_rect(dx, dy, -12.0, -6.0, 12.0, 8.0, 1.6);
            let dpad = glyph_seg(dx, dy, -9.0, 1.0, -3.0, 1.0, 1.4) || glyph_seg(dx, dy, -6.0, -2.0, -6.0, 4.0, 1.4);
            let buttons = ((dx - 5.0).powi(2) + (dy + 0.5).powi(2)).sqrt() < 1.8 || ((dx - 8.5).powi(2) + (dy - 3.0).powi(2)).sqrt() < 1.8;
            body || dpad || buttons
        }
        "RefreshCw" => {
            // Circular arrow with a gap in the upper right
            let arc = (dist - 8.0).abs() < 1.6 && !(dx > 1.5 && dy < -1.5);
            let head = glyph_seg(dx, dy, 8.0, -7.0, 4.5, -3.0, 1.5) || glyph_seg(dx, dy, 8.0, -7.0, 11.5, -3.0, 1.5);
            arc || head
        }
        "Globe" => {
            let ball = (dist - 11.0).abs() < 1.5;
            let meridian = (((dx / 5.5).powi(2) + (dy / 11.0).powi(2)) - 1.0).abs() < 0.3;
            let equator = dy.abs() < 1.2 && dx.abs() <= 11.0;
            ball || meridian || equator
        }
        "Keyboard" => {
            let body = glyph_rect(dx, dy, -12.0, -7.0, 12.0, 8.0, 1.6);
            let keys = [-7.0_f32, -3.5, 0.0, 3.5, 7.0]
                .iter()
                .any(|kx| ((dx - kx).powi(2) + (dy + 2.5).powi(2)).sqrt() < 1.1 || ((dx - kx).powi(2) + (dy - 0.5).powi(2)).sqrt() < 1.1);
            let space = glyph_seg(dx, dy, -5.0, 4.0, 5.0, 4.0, 1.2);
            body || keys || space
        }
        "Scissors" => {
            // Two finger loops on left + crossed blades to right
            let loop1 = ((dx + 7.5).powi(2) + (dy - 5.5).powi(2)).sqrt();
            let loop2 = ((dx + 7.5).powi(2) + (dy + 5.5).powi(2)).sqrt();
            let rings = (loop1 - 4.0).abs() < 1.5 || (loop2 - 4.0).abs() < 1.5;
            let blade1 = dx >= -4.5 && dx <= 11.5 && (dy - (-0.65 * dx + 1.0)).abs() < 1.4;
            let blade2 = dx >= -4.5 && dx <= 11.5 && (dy - (0.65 * dx - 1.0)).abs() < 1.4;
            rings || blade1 || blade2
        }
        "Camera" => {
            // Camera body + top viewfinder hump + center lens ring
            let body = dx.abs() <= 12.0
                && dy >= -6.5
                && dy <= 10.0
                && (dx.abs() >= 10.3 || (dy - 10.0).abs() < 1.7 || (dy - (-6.5)).abs() < 1.7);
            let top_bump = dx.abs() <= 5.5 && dy >= -10.0 && dy <= -6.5 && ((dy - (-10.0)).abs() < 1.6 || (dx.abs() - 5.5).abs() < 1.6);
            let lens = ((dx * dx + (dy - 2.0).powi(2)).sqrt() - 4.5).abs() < 1.6;
            body || top_bump || lens
        }
        "Terminal" => {
            // `>_` prompt inside terminal
            let chevron = dx >= -10.5
                && dx <= -2.0
                && dy >= -7.0
                && dy <= 5.0
                && (((dy + 1.0).abs() - (-(dx + 2.0) * 0.7)).abs() < 1.6);
            let underscore = dx >= 1.0 && dx <= 10.5 && (dy - 5.5).abs() < 1.6;
            chevron || underscore
        }
        "Monitor" => {
            // Desktop monitor screen + stand neck + base
            let screen = dx.abs() <= 12.0
                && dy >= -10.0
                && dy <= 5.5
                && (dx.abs() >= 10.3 || (dy - (-10.0)).abs() < 1.7 || (dy - 5.5).abs() < 1.7);
            let neck = dx.abs() < 1.5 && dy >= 5.5 && dy <= 10.0;
            let base = dx.abs() <= 6.5 && (dy - 10.2).abs() < 1.5;
            screen || neck || base
        }
        "Music" => {
            // Beamed eighth notes
            let note1 = ((dx + 5.5).powi(2) + (dy - 6.5).powi(2)).sqrt() < 3.4;
            let note2 = ((dx - 5.5).powi(2) + (dy - 4.5).powi(2)).sqrt() < 3.4;
            let stem1 = (dx + 2.8).abs() < 1.2 && dy >= -8.0 && dy <= 6.5;
            let stem2 = (dx - 8.2).abs() < 1.2 && dy >= -10.0 && dy <= 4.5;
            let beam = dx >= -3.0 && dx <= 8.5 && (dy - (-8.5 - (dx + 3.0) * 0.15)).abs() < 1.8;
            note1 || note2 || stem1 || stem2 || beam
        }
        "Zap" => {
            // Lightning bolt
            let upper = dy >= -11.0 && dy <= 0.5 && (dx - (-dy * 0.55 - 2.0)).abs() < 2.0;
            let mid = dy.abs() < 1.5 && dx.abs() <= 5.5;
            let lower = dy >= -0.5 && dy <= 11.0 && (dx - (-dy * 0.55 + 2.0)).abs() < 2.0;
            upper || mid || lower
        }
        "Pause" => {
            let left = (-8.0..=-3.0).contains(&dx) && dy.abs() <= 9.0;
            let right = (3.0..=8.0).contains(&dx) && dy.abs() <= 9.0;
            left || right
        }
        "Play" => dx >= -6.0 && dx <= 8.0 && (dx + 6.0) >= dy.abs() * 0.9,
        "Mic" => {
            let capsule = dx.abs() <= 4.5 && (-11.0..=1.0).contains(&dy) && (dx.abs() >= 2.8 || (dy + 11.0).abs() < 1.8 || (dy - 1.0).abs() < 1.8);
            let u_cup = (-2.0..=5.5).contains(&dy) && (dist - 7.8).abs() < 1.6;
            let stand = (dx.abs() < 1.2 && (5.5..=10.5).contains(&dy)) || ((9.5..=11.2).contains(&dy) && dx.abs() <= 5.5);
            capsule || u_cup || stand
        }
        _ => {
            // Generic 2x2 grid icon
            let gx = (dx.abs() - 5.5).abs();
            let gy = (dy.abs() - 5.5).abs();
            gx <= 3.5 && gy <= 3.5 && (gx >= 2.0 || gy >= 2.0)
        }
    }
}

/// Rotates a panel bitmap into the orientation the device expects.
/// 90° and 270° are applied only when the panel is square, so a wide touch strip
/// is not sent with its width and height swapped.
pub(crate) fn orient_for_panel(img: RgbImage, rotation: u16) -> RgbImage {
    let (w, h) = img.dimensions();
    let r = rotation % 360;
    if (r == 90 || r == 270) && w != h {
        return img;
    }
    match r {
        90 => image::imageops::rotate90(&img),
        180 => image::imageops::rotate180(&img),
        270 => image::imageops::rotate270(&img),
        _ => img,
    }
}

fn paint_level_meter(img: &mut RgbImage, percent: u8, accent: [u8; 3]) {
    let width = img.width() as i32;
    let height = img.height() as i32;
    let segments = 8;
    let gap = 2;
    let bar_w = (width * 72 / 100).max(32);
    let seg_w = ((bar_w - gap * (segments - 1)) / segments).max(2);
    let used = seg_w * segments + gap * (segments - 1);
    let start_x = (width - used) / 2;
    let y = height - 7;
    let filled = ((percent.min(100) as i32) * segments + 99) / 100;
    for index in 0..segments {
        let x0 = start_x + index * (seg_w + gap);
        let color = if index < filled {
            Rgb([accent[0], accent[1], accent[2]])
        } else {
            Rgb([39, 45, 61])
        };
        for py in y..(y + 3).min(height) {
            for px in x0..(x0 + seg_w).min(width) {
                if px >= 0 && py >= 0 {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

/// Generates a hardware-ready baseline JPEG (`width x height`, e.g., `112x112` on Ajazz AKP05E Pro)
/// that faithfully renders the DeckForge UI keycap:
/// 1. Dark gradient card background with outer rounded accent border
/// 2. Rounded inner icon badge container (`38x38`) with translucent accent fill & crisp border
/// 3. Vector-matched glyph for `icon_name` (`Radio`, `Clapperboard`, `Film`, `MicOff`, `Volume2`, `Disc`, `Scissors`, `Camera`, `Terminal`, `Monitor`, etc.)
/// 4. Crisp centered white title text (`"Start Stream"`, `"Scene +"`, `"Mute Mic"`, etc.) at the bottom!
pub fn encode_procedural_key_jpeg(
    width: u32,
    height: u32,
    accent_hex: &str,
    bg_hex: &str,
    icon_name: &str,
    title: &str,
    _key_index: u8,
    rotation: u16,
    level: Option<u8>,
) -> Vec<u8> {
    let accent = parse_hex_color(accent_hex, [139, 92, 246]);
    let bg = parse_hex_color(bg_hex, [21, 25, 37]);

    let mut img: RgbImage = ImageBuffer::new(width, height);
    let cx = (width as f32) * 0.5;
    let icon_cy = (height as f32) * 0.37;
    let max_r = (width.min(height) as f32) * 0.72;

    // Inner icon badge dimensions (matching the UI screenshot's rounded square badge around each icon)
    let badge_half = (width.min(height) as f32) * 0.19; // ~21px half-width = 42x42 badge on 112x112

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32;
            let fy = y as f32;
            let dx = fx - cx;
            let dy = fy - icon_cy;
            let dist = (dx * dx + dy * dy).sqrt();

            // Subtle radial background glow matching the DeckForge UI card
            let t = (1.0 - (dist / max_r)).clamp(0.0, 1.0);
            let glow = t * t * 0.38;

            let is_outer_border = x < 2 || y < 2 || x >= width - 2 || y >= height - 2;

            // Rounded rectangle distance for inner badge
            let qx = dx.abs() - (badge_half - 6.0);
            let qy = dy.abs() - (badge_half - 6.0);
            let badge_sdf = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt()
                + qx.max(qy).min(0.0)
                - 6.0;

            let in_badge_fill = badge_sdf <= 0.0;
            let in_badge_border = badge_sdf.abs() < 1.25;
            let in_icon_glyph = badge_sdf < -2.0 && icon_stroke_hit(icon_name, dx, dy);

            let base_r = ((bg[0] as f32) * (1.0 - glow) + (accent[0] as f32) * glow) as u8;
            let base_g = ((bg[1] as f32) * (1.0 - glow) + (accent[1] as f32) * glow) as u8;
            let base_b = ((bg[2] as f32) * (1.0 - glow) + (accent[2] as f32) * glow) as u8;

            let pixel = if is_outer_border {
                Rgb([
                    ((base_r as u16 + accent[0] as u16) / 2) as u8,
                    ((base_g as u16 + accent[1] as u16) / 2) as u8,
                    ((base_b as u16 + accent[2] as u16) / 2) as u8,
                ])
            } else if in_icon_glyph {
                Rgb([
                    accent[0].saturating_add(45),
                    accent[1].saturating_add(45),
                    accent[2].saturating_add(45),
                ])
            } else if in_badge_border {
                Rgb([
                    ((base_r as u16 + accent[0] as u16 * 2) / 3) as u8,
                    ((base_g as u16 + accent[1] as u16 * 2) / 3) as u8,
                    ((base_b as u16 + accent[2] as u16 * 2) / 3) as u8,
                ])
            } else if in_badge_fill {
                Rgb([
                    ((base_r as u16 * 3 + accent[0] as u16) / 4) as u8,
                    ((base_g as u16 * 3 + accent[1] as u16) / 4) as u8,
                    ((base_b as u16 * 3 + accent[2] as u16) / 4) as u8,
                ])
            } else {
                Rgb([base_r, base_g, base_b])
            };

            img.put_pixel(x, y, pixel);
        }
    }

    if let Some(percent) = level {
        paint_level_meter(&mut img, percent, accent);
    }

    // Draw centered white title text below the icon badge (matching the UI screenshot!)
    if !title.trim().is_empty() {
        let lift = if level.is_some() { 8 } else { 0 };
        let scale = if title.len() <= 8 && width >= 96 { 2 } else { 1 };
        let text_y = (height as i32) - if scale == 2 { 24 } else { 18 } - lift;
        if scale == 1 && width >= 96 {
            // For longer titles like "Start Stream" or "Show Desktop", split into 2 lines at scale=2 if space-separated,
            // or render crisp 1-line scale=1/2 so it is super readable on the physical LCD!
            let words: Vec<&str> = title.split_whitespace().collect();
            if words.len() == 2 && words[0].len() <= 7 && words[1].len() <= 7 {
                draw_text_centered(
                    &mut img,
                    words[0],
                    (width / 2) as i32,
                    (height as i32) - 31 - lift,
                    2,
                    Rgb([244, 245, 250]),
                );
                draw_text_centered(
                    &mut img,
                    words[1],
                    (width / 2) as i32,
                    (height as i32) - 15 - lift,
                    2,
                    Rgb([244, 245, 250]),
                );
            } else {
                draw_text_centered(
                    &mut img,
                    title,
                    (width / 2) as i32,
                    text_y,
                    1,
                    Rgb([244, 245, 250]),
                );
            }
        } else {
            draw_text_centered(
                &mut img,
                title,
                (width / 2) as i32,
                text_y,
                scale,
                Rgb([244, 245, 250]),
            );
        }
    }

    let oriented = orient_for_panel(img, rotation);
    let mut out = Cursor::new(Vec::with_capacity(8192));
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 92);
    let _ = encoder.encode_image(&oriented);
    out.into_inner()
}

/// Renders the 800x100 4-zone Touch Strip JPEG matching the DeckForge UI (`SPOTIFY`, `CPU LOAD`, `MIXER`, `MIC MUTE`)
/// rotated 180° for the Ajazz AKP05E Pro secondary LCD strip.
pub fn encode_touch_strip_jpeg(
    width: u32,
    height: u32,
    zones: &[(String, String, String)], // (title, value_text, accent_hex)
    rotation: u16,
) -> Vec<u8> {
    let mut img: RgbImage = ImageBuffer::new(width, height);
    let bg = [13u8, 16u8, 24u8];
    let card_bg = [20u8, 25u8, 36u8];
    let card_border = [38u8, 45u8, 64u8];

    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x, y, Rgb(bg));
        }
    }

    let zone_count = zones.len().max(1) as u32;
    let zone_w = width / zone_count;

    for (idx, (title, value_text, accent_hex)) in zones.iter().enumerate() {
        let zx0 = (idx as u32) * zone_w + 8;
        let zx1 = ((idx as u32 + 1) * zone_w).saturating_sub(8);
        let zy0 = 10u32;
        let zy1 = height.saturating_sub(10);
        let accent = parse_hex_color(accent_hex, [139, 92, 246]);

        for y in zy0..zy1 {
            for x in zx0..zx1 {
                let is_border = x == zx0 || x == zx1 - 1 || y == zy0 || y == zy1 - 1;
                if is_border {
                    img.put_pixel(x, y, Rgb(card_border));
                } else {
                    img.put_pixel(x, y, Rgb(card_bg));
                }
            }
        }

        let center_x = ((zx0 + zx1) / 2) as i32;
        draw_text_centered(&mut img, title, center_x, (zy0 + 14) as i32, 2, Rgb([148, 156, 179]));
        draw_text_centered(
            &mut img,
            value_text,
            center_x,
            (zy0 + 44) as i32,
            2,
            Rgb([
                accent[0].saturating_add(30),
                accent[1].saturating_add(30),
                accent[2].saturating_add(30),
            ]),
        );
    }

    let oriented = orient_for_panel(img, rotation);
    let mut out = Cursor::new(Vec::with_capacity(16384));
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
    let _ = encoder.encode_image(&oriented);
    out.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_and_brightness_packets_match_sdlibrary1_disassembly() {
        let han = build_handshake_packet();
        assert_eq!(han.len(), 1025);
        assert_eq!(han[0], 0x00);
        assert_eq!(&han[1..6], b"CRT\0\0");
        assert_eq!(&han[6..9], b"HAN");
        assert_eq!(han[11], 0x01);
        let wake = build_screen_on_packet();
        assert_eq!(&wake[6..9], b"DIS");

        let lig = build_brightness_packet(85);
        assert_eq!(lig.len(), 1025);
        assert_eq!(&lig[1..6], b"CRT\0\0");
        assert_eq!(&lig[6..9], b"LIG");
        assert_eq!(lig[11], 85);

        let lig_clamped = build_brightness_packet(250);
        assert_eq!(lig_clamped[11], 100);
    }

    #[test]
    fn akp05e_image_slots_match_the_physical_face() {
        assert_eq!(
            (0..5).map(akp05e_key_image_slot).collect::<Vec<_>>(),
            vec![11, 12, 13, 14, 15]
        );
        assert_eq!(
            (5..10).map(akp05e_key_image_slot).collect::<Vec<_>>(),
            vec![6, 7, 8, 9, 10]
        );
        assert_eq!(
            (0..4).map(akp05e_touch_zone_slot).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn test_key_image_bat_chunking_and_stp_commit() {
        let fake_jpeg = vec![0xFFu8; 2100]; // 3 chunks of 1024B (1024 + 1024 + 52)
        let pkts = build_key_image_packets(3, &fake_jpeg);
        // 1 header (BAT) + 3 payload chunks + 1 finish (STP) = 5 packets
        assert_eq!(pkts.len(), 5);
        assert_eq!(&pkts[0][6..9], b"BAT");
        assert_eq!(pkts[0][11], (2100u16 >> 8) as u8);
        assert_eq!(pkts[0][12], (2100u16 & 0xFF) as u8);
        assert_eq!(pkts[0][13], 3);
        assert_eq!(&pkts[4][6..9], b"STP");
    }

    #[test]
    fn test_parse_ack_ok_input_report() {
        let mut report = [0u8; 513];
        report[0] = 0x00;
        report[1..4].copy_from_slice(b"ACK");
        report[6..8].copy_from_slice(b"OK");
        report[10] = 0x01; // Key #1 (1-based)
        report[11] = 0x01; // Pressed
        let evt = parse_input_report("akp05e-0300-3013", &report, 12345).expect("valid key press");
        assert_eq!(evt.kind, InputControlKind::LcdKey);
        assert_eq!(evt.control_index, 0);
        assert!(evt.pressed);
    }

    #[test]
    fn command_ack_is_not_a_button() {
        let mut report = [0u8; 32];
        report[0..3].copy_from_slice(b"ACK");
        report[5..7].copy_from_slice(b"OK");
        assert!(parse_input_report("ajazz-akp05e-pro", &report, 1).is_none());
    }

    #[test]
    fn spotify_touch_and_volume_knob_have_their_own_codes() {
        let mut touch = [0u8; 32];
        touch[0..3].copy_from_slice(b"ACK");
        touch[9] = 0x40;
        touch[10] = 0x00;
        let evt = parse_input_report("ajazz-akp05e-pro", &touch, 1).unwrap();
        assert_eq!(evt.kind, InputControlKind::TouchStripTap);
        assert_eq!(evt.control_index, 0);
        assert!(evt.pressed);

        let mut knob = [0u8; 32];
        knob[0..3].copy_from_slice(b"ACK");
        knob[9] = 0xA1;
        let turn = parse_input_report("ajazz-akp05e-pro", &knob, 1).unwrap();
        assert_eq!(turn.kind, InputControlKind::RotaryKnobTurnCw);
        assert_eq!(turn.control_index, 0);
    }

    #[test]
    fn touchbar_tap_on_the_left_is_the_spotify_zone() {
        let mut down = [0u8; 32];
        down[0..3].copy_from_slice(b"ACK");
        down[5..7].copy_from_slice(b"OK");
        down[10] = 0x01;
        down[12] = 40;
        let down_evt = parse_input_report("ajazz-akp05e-pro", &down, 1).unwrap();
        assert_eq!(down_evt.kind, InputControlKind::TouchStripPoint);
        assert_eq!(down_evt.x, 40);

        let mut gesture = StripGesture::default();
        assert!(resolve_strip_point(&mut gesture, &down_evt).is_none());
        down[10] = 0x00;
        let release = parse_input_report("ajazz-akp05e-pro", &down, 2).unwrap();
        let tap = resolve_strip_point(&mut gesture, &release).unwrap();
        assert_eq!(tap.kind, InputControlKind::TouchStripTap);
        assert_eq!(tap.control_index, 0);
    }

    #[test]
    fn a_finger_drag_changes_the_page() {
        let mut gesture = StripGesture::default();
        assert_eq!(observe_strip_point(&mut gesture, 1, 40), StripGestureResult::Ignore);
        assert_eq!(observe_strip_point(&mut gesture, 2, 420), StripGestureResult::Ignore);
        assert_eq!(observe_strip_point(&mut gesture, 0, 460), StripGestureResult::SwipeNext);
        assert_eq!(observe_strip_point(&mut gesture, 1, 700), StripGestureResult::Ignore);
        assert_eq!(observe_strip_point(&mut gesture, 0, 80), StripGestureResult::SwipePrev);
    }

    #[test]
    fn button_mode_keeps_the_key_images() {
        let pkt = build_button_mode_packet();
        assert_eq!(&pkt[6..9], b"MOD");
        assert_eq!(pkt[11], b'1');
        assert_ne!(pkt[11], build_touchbar_mode_packet()[11]);
    }

    #[test]
    fn touchbar_mode_reports_the_finger() {
        let pkt = build_touchbar_mode_packet();
        assert_eq!(&pkt[6..9], b"MOD");
        assert_eq!(pkt[11], b'2');

        let mut move_pkt = [0u8; 32];
        move_pkt[0..3].copy_from_slice(b"ACK");
        move_pkt[5..8].copy_from_slice(b"ARX");
        move_pkt[10] = 0x01;
        move_pkt[11] = 0x2C;
        let evt = parse_input_report("ajazz-akp05e-pro", &move_pkt, 1).unwrap();
        assert_eq!(evt.kind, InputControlKind::TouchStripPoint);
        assert_eq!(evt.state_code, 2);
        assert_eq!(evt.x, 300);
    }

    #[test]
    fn strip_swipe_is_left_or_right() {
        let mut left = [0u8; 32];
        left[0..3].copy_from_slice(b"ACK");
        left[9] = 0x38;
        let evt = parse_input_report("ajazz-akp05e-pro", &left, 1).unwrap();
        assert_eq!(evt.kind, InputControlKind::TouchStripSwipe);
        assert_eq!(evt.control_index, 0);
        assert!(evt.pressed);

        left[9] = 0x39;
        let right = parse_input_report("ajazz-akp05e-pro", &left, 2).unwrap();
        assert_eq!(right.control_index, 1);
    }

    const GLYPHS: &[&str] = &[
        "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "X", "Plus", "Copy", "Clipboard", "Undo", "Redo", "Save",
        "Smile", "Settings", "Folder", "Gamepad2", "RefreshCw", "Globe", "Keyboard", "Mic", "Camera", "Disc",
    ];

    fn glyph_pixels(name: &str) -> usize {
        let mut hits = 0;
        for yi in -18..=18 {
            for xi in -18..=18 {
                if icon_stroke_hit(name, xi as f32, yi as f32) {
                    hits += 1;
                }
            }
        }
        hits
    }

    #[test]
    fn named_glyphs_are_not_the_generic_fallback() {
        let generic = glyph_pixels("definitely-not-an-icon");
        for name in GLYPHS {
            let hits = glyph_pixels(name);
            assert!(hits >= 40, "glyph {name} draws almost nothing ({hits} px)");
            let same_as_generic = (-18..=18).all(|yi| {
                (-18..=18).all(|xi| {
                    icon_stroke_hit(name, xi as f32, yi as f32)
                        == icon_stroke_hit("definitely-not-an-icon", xi as f32, yi as f32)
                })
            });
            assert!(!same_as_generic, "glyph {name} still renders the generic grid ({generic} px)");
        }
    }

    /// Writes a contact sheet of every glyph to the temp dir so the shapes can be eyeballed.
    #[test]
    #[ignore]
    fn render_glyph_contact_sheet() {
        let cell = 96u32;
        let cols = 7u32;
        let rows = (GLYPHS.len() as u32).div_ceil(cols);
        let mut img = RgbImage::from_pixel(cols * cell, rows * cell, Rgb([16, 19, 29]));
        for (i, name) in GLYPHS.iter().enumerate() {
            let ox = (i as u32 % cols) * cell;
            let oy = (i as u32 / cols) * cell;
            for py in 0..cell {
                for px in 0..cell {
                    let scale = 36.0 / cell as f32;
                    let dx = (px as f32 - cell as f32 / 2.0) * scale;
                    let dy = (py as f32 - cell as f32 / 2.0) * scale;
                    if icon_stroke_hit(name, dx, dy) {
                        img.put_pixel(ox + px, oy + py, Rgb([139, 92, 246]));
                    }
                }
            }
        }
        let path = std::env::temp_dir().join("deckforge-glyph-sheet.png");
        img.save(&path).unwrap();
        println!("wrote {}", path.display());
    }

    #[test]
    fn test_knob_led_and_reset_packets() {
        let colors = [
            [0xFF, 0x00, 0x00],
            [0x00, 0xFF, 0x00],
            [0x00, 0x00, 0xFF],
            [0xFF, 0x10, 0x80],
        ];
        let pkt = build_knob_led_packet(&colors);
        assert_eq!(pkt.len(), HID_REPORT_LEN);
        assert_eq!(&pkt[0..11], &[0x00, b'C', b'R', b'T', 0x00, 0x00, b'S', b'E', b'T', b'L', b'B']);
        assert_eq!(&pkt[11..23], &[0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x10, 0x80]);
        assert!(pkt[23..].iter().all(|&b| b == 0));

        let reset = build_reset_leds_packet();
        assert_eq!(reset.len(), HID_REPORT_LEN);
        assert_eq!(&reset[0..11], &[0x00, b'C', b'R', b'T', 0x00, 0x00, b'D', b'E', b'L', b'E', b'D']);
        assert!(reset[11..].iter().all(|&b| b == 0));
    }
}
