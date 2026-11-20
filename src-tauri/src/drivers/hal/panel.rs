use super::catalog::{now_ms, DeviceDescriptor, DiagnosticEntry};
use super::io::{debug_flag, deck_write};
use super::publish::{ensure_akp_io_target, nudged_brightness, panel_brightness};
use crate::drivers::akp05e_protocol::{
    build_brightness_packet, build_knob_led_packet, build_led_brightness_packet,
    build_led_sleep_mode_packet, build_reset_leds_packet, parse_hex_color,
};

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

    ensure_akp_io_target(dev)?;

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
    ensure_akp_io_target(dev)?;
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

    ensure_akp_io_target(dev)?;

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

    ensure_akp_io_target(dev)?;

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

    ensure_akp_io_target(dev)?;

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

    ensure_akp_io_target(dev)?;

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
    ensure_akp_io_target(dev)?;
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
    ensure_akp_io_target(dev)?;
    let pkt = build_brightness_packet(percent.min(100));
    deck_write(vec![(pkt.to_vec(), 0)])
}
