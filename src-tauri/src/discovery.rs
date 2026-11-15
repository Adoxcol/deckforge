use super::*;

// Serialize discovery without blocking commands that only need app state.
static SCAN: Mutex<()> = Mutex::new(());

pub(crate) fn refresh(shared: &SharedState) -> (Vec<DeviceDescriptor>, bool) {
    refresh_with(shared, scan_usb_devices)
}

pub(crate) fn refresh_with(
    shared: &SharedState,
    scan: impl FnOnce(&mut [DeviceDescriptor], &mut Vec<DiagnosticEntry>),
) -> (Vec<DeviceDescriptor>, bool) {
    let _scan = SCAN.lock_safe();
    let mut devices = shared.lock_safe().devices.clone();
    let mut diagnostics = Vec::new();
    scan(&mut devices, &mut diagnostics);
    let mut guard = shared.lock_safe();
    let connected = |devices: &[DeviceDescriptor]| devices.iter().any(|d| d.connected && d.protocol_family.starts_with("CRT"));
    let was_connected = connected(&guard.devices);
    for found in devices {
        if let Some(current) = guard.devices.iter_mut().find(|d| d.id == found.id) {
            // Merge discovery fields only: brightness and other settings may
            // have changed while Windows enumerated USB devices.
            current.connected = found.connected;
            current.is_simulator = found.is_simulator;
            current.hid_path = found.hid_path;
            current.last_seen_ms = found.last_seen_ms;
            current.serial_number = found.serial_number;
        }
    }
    guard.diagnostics.extend(diagnostics);
    let excess = guard.diagnostics.len().saturating_sub(80);
    guard.diagnostics.drain(..excess);
    let newly_connected = !was_connected && connected(&guard.devices);
    (guard.devices.clone(), newly_connected)
}
