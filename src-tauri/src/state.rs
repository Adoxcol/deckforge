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
    pub applied_context: Option<DeckContextEvent>,
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
    #[serde(default)]
    pub cpu_percent: Option<u8>,
    #[serde(default)]
    pub ram_percent: Option<u8>,
    /// Local time as HH:MM.
    #[serde(default)]
    pub clock: String,
    /// Album art of what is playing, as the image bytes Windows hands out.
    #[serde(skip)]
    pub media_art: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckContextEvent {
    pub profile_id: String,
    pub page_index: usize,
    pub message: String,
}

/// The exact bindings whose images were successfully sent to the device.
#[derive(Clone, Serialize, Deserialize)]
pub struct AppliedLayout {
    pub profile_id: String,
    pub page_index: usize,
    pub page: ProfilePage,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStatus {
    pub deck: DeckStatus,
    pub applied: Option<AppliedPageStatus>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedPageStatus {
    pub profile_name: String,
    pub page_name: String,
    pub matches_editor: bool,
    pub layout_ready: bool,
}

impl AppliedLayout {
    fn matches_editor(&self, profile_id: &str, page: &ProfilePage) -> bool {
        self.profile_id == profile_id
            && serde_json::to_value(&self.page).ok().zip(serde_json::to_value(page).ok())
                .is_some_and(|(applied, editor)| applied == editor)
    }
}

#[derive(Serialize, Deserialize)]
struct SavedAppliedLayout {
    version: u32,
    layout: AppliedLayout,
}

pub(crate) fn load_applied_layout(path: &std::path::Path) -> Result<Option<AppliedLayout>, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.to_string()),
    };
    let saved: SavedAppliedLayout = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if saved.version != 1 || saved.layout.profile_id.is_empty() || saved.layout.page.id.is_empty() {
        return Err("Invalid or unsupported saved deck layout".into());
    }
    Ok(Some(saved.layout))
}

fn save_applied_layout(path: &std::path::Path, layout: &AppliedLayout) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string(&SavedAppliedLayout { version: 1, layout: layout.clone() })
        .map_err(|e| e.to_string())?;
    db::write_atomic(path, &raw)
}

// Serialize layout changes and background redraws without holding SharedState during USB I/O.
pub(crate) static DISPLAY_WRITE: Mutex<()> = Mutex::new(());

pub struct ManagedAppState {
    pub devices: Vec<DeviceDescriptor>,
    pub profiles: Vec<Profile>,
    pub active_profile_id: String,
    pub active_page_index: usize,
    /// The profile and page shown before the last page action, for the Back key.
    pub previous_page: Option<(String, usize)>,
    pub applied_layout: Option<AppliedLayout>,
    pub applied_layout_path: Option<std::path::PathBuf>,
    pub layout_ready: bool,
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

        let profiles = load_saved_profiles().unwrap_or_else(|err| {
            diagnostics.push(DiagnosticEntry {
                id: "diag-profiles-load".into(), timestamp_ms: now_ms(), level: "error".into(),
                device_id: "system".into(), stage: "RESTORE_PROFILES".into(),
                message: format!("Could not load profiles: {err}. Showing defaults; saving is blocked until the original file is repaired or moved aside."),
                packet_hex: None,
            });
            db::default_profiles()
        });
        let applied_layout_path = db::profiles_store_path().with_file_name("applied-layout.json");
        let applied_layout = load_applied_layout(&applied_layout_path).unwrap_or_else(|err| {
            diagnostics.push(DiagnosticEntry {
                id: "diag-layout-load".into(), timestamp_ms: now_ms(), level: "warning".into(),
                device_id: "ajazz-akp05e-pro".into(), stage: "RESTORE_LAYOUT".into(),
                message: format!("Could not restore the last deck layout: {err}"), packet_hex: None,
            });
            None
        });
        let active_profile_id = applied_layout.as_ref().map(|a| a.profile_id.clone())
            .or_else(|| profiles.first().map(|p| p.id.clone())).unwrap_or_default();
        let active_page_index = applied_layout.as_ref().map(|a| a.page_index).unwrap_or(0);
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
            active_profile_id,
            active_page_index,
            previous_page: None,
            applied_layout,
            applied_layout_path: Some(applied_layout_path),
            layout_ready: false,
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
            face: DeckFaceState::default(),
            obs: ObsSettings::load(),
            sample_after_ms: 0,
            auto_switch_hold_until_ms: 0,
        }
    }
}

impl ManagedAppState {
    pub(crate) fn applied_page_status(&self, device_id: &str, profile_id: &str, page: &ProfilePage) -> Option<AppliedPageStatus> {
        let applied = self.applied_layout.as_ref()?;
        Some(AppliedPageStatus {
            profile_name: self.profiles.iter().find(|p| p.id == applied.profile_id)
                .map(|p| p.name.clone()).unwrap_or_else(|| "Removed profile".into()),
            page_name: applied.page.name.clone(),
            matches_editor: self.playback_device_id == device_id && applied.matches_editor(profile_id, page),
            layout_ready: self.layout_ready,
        })
    }

    pub(crate) fn persist_applied_layout(&self) -> Result<(), String> {
        match (&self.applied_layout_path, &self.applied_layout) {
            (Some(path), Some(layout)) => save_applied_layout(path, layout),
            _ => Ok(()),
        }
    }

    pub(crate) fn applied_editor_context(&self) -> Option<DeckContextEvent> {
        let applied = self.applied_layout.as_ref()?;
        let profile = self.profiles.iter().find(|p| p.id == applied.profile_id)?;
        // Editor pages may have been reordered since this snapshot was applied.
        let page_index = profile.pages.iter().position(|p| p.id == applied.page.id)?;
        Some(DeckContextEvent {
            profile_id: profile.id.clone(), page_index,
            message: "Restored last applied layout".into(),
        })
    }
}

pub type SharedState = Arc<Mutex<ManagedAppState>>;

/// Locks a mutex even when another thread panicked while holding it. A poisoned lock
/// used to make every later command fail and every key press do nothing until restart.
pub trait LockExt<T> {
    fn lock_safe(&self) -> std::sync::MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_safe(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod applied_status_tests {
    use super::*;

    #[test]
    fn applied_status_compares_content_not_just_page_ids() {
        let profile = db::default_profiles().remove(0);
        let page = profile.pages[0].clone();
        let applied = AppliedLayout { profile_id: profile.id.clone(), page_index: 0, page: page.clone() };
        assert!(applied.matches_editor(&profile.id, &page));
        let mut draft = page.clone();
        draft.keys[0].title = "Unsaved draft".into();
        assert!(!applied.matches_editor(&profile.id, &draft));
        // Saving the draft does not mutate the successfully applied snapshot.
        let saved: ProfilePage = serde_json::from_str(&serde_json::to_string(&draft).unwrap()).unwrap();
        assert!(!applied.matches_editor(&profile.id, &saved));
        assert!(applied.matches_editor(&profile.id, &page)); // Undo restores equality.
    }

    #[test]
    fn applied_status_requires_the_same_profile_and_page() {
        let profile = db::default_profiles().remove(0);
        let page = profile.pages[0].clone();
        let applied = AppliedLayout { profile_id: profile.id.clone(), page_index: 0, page: page.clone() };
        assert!(!applied.matches_editor("other-profile", &page));
        let mut other_page = page.clone();
        other_page.id = "other-page".into();
        assert!(!applied.matches_editor(&profile.id, &other_page));
    }
}
