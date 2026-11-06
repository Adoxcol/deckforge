use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullBootstrapPayload {
    pub devices: Vec<DeviceDescriptor>,
    pub profiles: Vec<Profile>,
    pub auto_switch_rules: Vec<AutoSwitchRule>,
    pub macros: Vec<MacroSequence>,
    pub diagnostics: Vec<DiagnosticEntry>,
    pub frame_cache_stats: FrameCacheStats,
    pub legacy_summary: LegacyStreamDockSummary,
    pub telemetry: SystemTelemetrySnapshot,
    pub saved_locally_at_ms: u64,
    pub applied_to_device_at_ms: Option<u64>,
    pub recent_input_events: Vec<HardwareInputEvent>,
    pub media_library: Vec<MediaPreview>,
    pub face: DeckFaceState,
    pub obs: ObsSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeckFaceState {
    pub media_playing: Option<bool>,
    pub media_title: String,
    pub mic_muted: Option<bool>,
    pub speaker_muted: Option<bool>,
    pub volume_percent: Option<u8>,
    pub game_mode: Option<bool>,
    pub game_mode_busy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckContextEvent {
    pub profile_id: String,
    pub page_index: usize,
    pub message: String,
}

pub struct ManagedAppState {
    pub devices: Vec<DeviceDescriptor>,
    pub profiles: Vec<Profile>,
    pub active_profile_id: String,
    pub active_page_index: usize,
    pub auto_switch_rules: Vec<AutoSwitchRule>,
    pub macros: Vec<MacroSequence>,
    pub diagnostics: Vec<DiagnosticEntry>,
    pub recent_input_events: Vec<HardwareInputEvent>,
    pub frame_cache: AntiFreezeFrameCache,
    pub saved_locally_at_ms: u64,
    pub applied_to_device_at_ms: Option<u64>,
    pub media: MediaStore,
    pub playback_slots: Vec<PlaybackSlot>,
    pub playback_device_id: String,
    pub usb_bus: Arc<Mutex<()>>,
    pub face: DeckFaceState,
    pub obs: ObsSettings,
    pub sample_after_ms: u64,
    pub auto_switch_hold_until_ms: u64,
}

impl Default for ManagedAppState {
    fn default() -> Self {
        let mut devices = builtin_device_catalog();
        let mut diagnostics = vec![DiagnosticEntry {
            id: "diag-init".to_string(),
            timestamp_ms: now_ms(),
            level: "info".to_string(),
            device_id: "system".to_string(),
            stage: "BOOT".to_string(),
            message: "DeckForge Rust Hardware & Action Service initialized (SHA-256 Frame Diffing + Win32 Input Engine enabled)".to_string(),
            packet_hex: None,
        }];
        scan_usb_devices(&mut devices, &mut diagnostics);

        let profiles = load_saved_profiles();
        let frame_cache = AntiFreezeFrameCache::new();
        let applied_to_device_at_ms = None;

        for d in &devices {
            if d.connected {
                diagnostics.push(DiagnosticEntry {
                    id: format!("diag-usb-{}", d.id),
                    timestamp_ms: now_ms(),
                    level: "success".to_string(),
                    device_id: d.id.clone(),
                    stage: "USB_DETECTED".to_string(),
                    message: format!(
                        "Verified hardware detected: {} ({}) Serial: {}",
                        d.model_name, d.vid_pid_hex, d.serial_number
                    ),
                    packet_hex: Some("CRT\\0\\0HAN ready (1024B payload)".to_string()),
                });
            }
        }

        Self {
            devices,
            profiles,
            active_profile_id: "profile-streaming".to_string(),
            active_page_index: 0,
            auto_switch_rules: load_auto_switch_rules(),
            macros: load_saved_macros(),
            diagnostics,
            recent_input_events: Vec::new(),
            frame_cache,
            saved_locally_at_ms: now_ms(),
            applied_to_device_at_ms,
            media: MediaStore::load(),
            playback_slots: Vec::new(),
            playback_device_id: String::new(),
            usb_bus: Arc::new(Mutex::new(())),
            face: DeckFaceState::default(),
            obs: ObsSettings::load(),
            sample_after_ms: 0,
            auto_switch_hold_until_ms: 0,
        }
    }
}

pub type SharedState = Arc<Mutex<ManagedAppState>>;
