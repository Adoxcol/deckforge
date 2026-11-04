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

/// `SDLibrary1.dll` @ `0x180024420` (`sendHandshakePack`): `"CRT\0\0HAN\0\0\x01"` (V4 AKP05E Pro sets payload[10] = 0x01)
pub fn build_handshake_packet() -> [u8; HID_REPORT_LEN] {
    let mut buf = crt_base_report(b"HAN");
    buf[11] = 0x01;
    buf
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

/// `SDLibrary1.dll` @ `0x180021280` / `0x180021ce5` (`readDataFromHidDevice`):
/// Parses an input report (e.g. 512/513-byte or 64/65-byte with Report ID) beginning with `"ACK\0\0OK"`.
pub fn parse_input_report(device_id: &str, report: &[u8], timestamp_ms: u64) -> Option<HardwareInputEvent> {
    let payload = if !report.is_empty() && report[0] == 0x00 {
        &report[1..]
    } else {
        report
    };

    if payload.len() < 11 {
        return None;
    }

    // Verify "ACK" at 0..3 and "OK" at 5..7
    if &payload[0..3] != b"ACK" || &payload[5..7] != b"OK" {
        return None;
    }

    let control_id = payload[9];
    let state_code = payload[10];

    let (kind, control_index, pressed) = match control_id {
        0x01..=0x20 => (
            InputControlKind::LcdKey,
            control_id.saturating_sub(1),
            state_code != 0x00,
        ),
        0x50..=0x5F => (
            InputControlKind::RotaryKnobTurnCcw,
            control_id - 0x50,
            true,
        ),
        0x60..=0x6F => (
            InputControlKind::RotaryKnobTurnCw,
            control_id - 0x60,
            true,
        ),
        0x30..=0x3F => (
            InputControlKind::RotaryKnobPush,
            control_id - 0x30,
            state_code != 0x00,
        ),
        0x40..=0x4F => (
            InputControlKind::TouchStripTap,
            control_id - 0x40,
            state_code != 0x00,
        ),
        _ => (InputControlKind::Unknown, control_id, state_code != 0x00),
    };

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
    })
}

/// Parses a `#RRGGBB` hex color string into `[u8; 3]`.
fn parse_hex_color(hex: &str, default: [u8; 3]) -> [u8; 3] {
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

fn draw_text_centered(
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
        "Disc" => {
            // Record disc: outer ring + inner core dot
            (dist - 11.0).abs() < 1.8 || dist < 3.6
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
        _ => {
            // Generic 2x2 grid icon
            let gx = (dx.abs() - 5.5).abs();
            let gy = (dy.abs() - 5.5).abs();
            gx <= 3.5 && gy <= 3.5 && (gx >= 2.0 || gy >= 2.0)
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

    // Draw centered white title text below the icon badge (matching the UI screenshot!)
    if !title.trim().is_empty() {
        let scale = if title.len() <= 8 && width >= 96 { 2 } else { 1 };
        let text_y = (height as i32) - if scale == 2 { 24 } else { 18 };
        if scale == 1 && width >= 96 {
            // For longer titles like "Start Stream" or "Show Desktop", split into 2 lines at scale=2 if space-separated,
            // or render crisp 1-line scale=1/2 so it is super readable on the physical LCD!
            let words: Vec<&str> = title.split_whitespace().collect();
            if words.len() == 2 && words[0].len() <= 7 && words[1].len() <= 7 {
                draw_text_centered(
                    &mut img,
                    words[0],
                    (width / 2) as i32,
                    (height as i32) - 31,
                    2,
                    Rgb([244, 245, 250]),
                );
                draw_text_centered(
                    &mut img,
                    words[1],
                    (width / 2) as i32,
                    (height as i32) - 15,
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

    let rotated = image::imageops::rotate180(&img);
    let mut out = Cursor::new(Vec::with_capacity(8192));
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 92);
    let _ = encoder.encode_image(&rotated);
    out.into_inner()
}

/// Renders the 800x100 4-zone Touch Strip JPEG matching the DeckForge UI (`SPOTIFY`, `CPU LOAD`, `MIXER`, `MIC MUTE`)
/// rotated 180° for the Ajazz AKP05E Pro secondary LCD strip.
pub fn encode_touch_strip_jpeg(
    width: u32,
    height: u32,
    zones: &[(String, String, String)], // (title, value_text, accent_hex)
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

    let rotated = image::imageops::rotate180(&img);
    let mut out = Cursor::new(Vec::with_capacity(16384));
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
    let _ = encoder.encode_image(&rotated);
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

        let lig = build_brightness_packet(85);
        assert_eq!(lig.len(), 1025);
        assert_eq!(&lig[1..6], b"CRT\0\0");
        assert_eq!(&lig[6..9], b"LIG");
        assert_eq!(lig[11], 85);

        let lig_clamped = build_brightness_packet(250);
        assert_eq!(lig_clamped[11], 100);
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
        let evt = parse_input_report("akp05e-0300-3013", &report, 12345).expect("valid ACK OK");
        assert_eq!(evt.kind, InputControlKind::LcdKey);
        assert_eq!(evt.control_index, 0);
        assert!(evt.pressed);
    }
}
