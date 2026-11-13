use super::*;
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDraft {
    version: u32,
    pub profiles: Vec<Profile>,
    base_profiles: Vec<Profile>,
    pub updated_at_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryDraft {
    draft: ProfileDraft,
    saved_profiles_changed: bool,
}

// A separate store: neither saving a draft nor discarding it changes saved/applied profiles.
static DRAFT_WRITE: Mutex<()> = Mutex::new(());
fn path() -> PathBuf { db::profiles_store_path().with_file_name("profile-draft.json") }

fn validate(profiles: &[Profile]) -> Result<(), String> {
    if profiles.is_empty() || profiles.iter().any(|p| p.pages.is_empty() || p.pages.iter().any(|page| page.keys.is_empty())) {
        return Err("Recovery draft has no usable profiles/pages/keys".into());
    }
    Ok(())
}

fn read_at(path: &Path) -> Result<Option<ProfileDraft>, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Could not read recovery draft: {e}")),
    };
    // null is an atomic tombstone, so a delayed/failed deletion cannot resurrect a draft.
    let draft: Option<ProfileDraft> = serde_json::from_str(&raw).map_err(|e| format!("Invalid recovery draft: {e}"))?;
    if let Some(draft) = &draft {
        if draft.version != 1 { return Err("Unsupported recovery draft version; stored draft was preserved".into()); }
        validate(&draft.profiles)?;
    }
    Ok(draft)
}

fn write_at(path: &Path, draft: Option<&ProfileDraft>) -> Result<(), String> {
    if draft.is_some() { read_at(path)?; } // Do not silently overwrite unreadable/future data.
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    let raw = serde_json::to_string(&draft).map_err(|e| e.to_string())?;
    db::write_atomic(path, &raw)
}

fn recovery(draft: Option<ProfileDraft>, saved: &[Profile]) -> Option<RecoveryDraft> {
    let draft = draft?;
    let content = |profiles: &[Profile]| {
        let mut value = serde_json::to_value(profiles).expect("profiles serialize");
        if let Some(profiles) = value.as_array_mut() {
            for profile in profiles { profile.as_object_mut().unwrap().remove("lastModifiedMs"); }
        }
        value
    };
    let current = content(saved);
    if content(&draft.profiles) == current { return None; }
    let saved_profiles_changed = content(&draft.base_profiles) != current;
    Some(RecoveryDraft { draft, saved_profiles_changed })
}

#[tauri::command]
pub fn get_profile_draft(state: State<'_, SharedState>) -> Result<Option<RecoveryDraft>, String> {
    let _write = DRAFT_WRITE.lock_safe();
    Ok(recovery(read_at(&path())?, &state.lock_safe().profiles))
}

#[tauri::command]
pub async fn save_profile_draft(profiles: Vec<Profile>, base_profiles: Vec<Profile>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate(&profiles)?;
        let _write = DRAFT_WRITE.lock_safe();
        write_at(&path(), Some(&ProfileDraft { version: 1, profiles, base_profiles, updated_at_ms: now_ms() }))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn discard_profile_draft() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _write = DRAFT_WRITE.lock_safe();
        write_at(&path(), None)
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("deckforge-draft-test-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap(); Self(dir)
        }
        fn path(&self) -> PathBuf { self.0.join("draft.json") }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            for entry in std::fs::read_dir(&self.0).unwrap().flatten() {
                if entry.path().is_dir() { let _ = std::fs::remove_dir(entry.path()); }
                else { let _ = std::fs::remove_file(entry.path()); }
            }
            let _ = std::fs::remove_dir(&self.0);
        }
    }
    fn draft() -> ProfileDraft {
        let base_profiles = db::default_profiles();
        let mut profiles = base_profiles.clone();
        profiles[0].pages[0].keys[0].title = "Unsaved title".into();
        ProfileDraft { version: 1, profiles, base_profiles, updated_at_ms: 123 }
    }
    #[test]
    fn restart_recovers_edits_and_detects_newer_saved_profiles() {
        let f = Fixture::new(); let d = draft();
        write_at(&f.path(), Some(&d)).unwrap();
        let r = recovery(read_at(&f.path()).unwrap(), &d.base_profiles).unwrap();
        assert_eq!(r.draft.profiles[0].pages[0].keys[0].title, "Unsaved title");
        assert!(!r.saved_profiles_changed);
        let mut changed = d.base_profiles.clone(); changed[0].last_modified_ms += 100;
        assert!(!recovery(read_at(&f.path()).unwrap(), &changed).unwrap().saved_profiles_changed);
        changed[0].name = "Changed elsewhere".into();
        assert!(recovery(read_at(&f.path()).unwrap(), &changed).unwrap().saved_profiles_changed);
        assert!(recovery(read_at(&f.path()).unwrap(), &d.profiles).is_none());
    }
    #[test]
    fn interrupted_write_preserves_previous_draft_and_discard_survives_reload() {
        let f = Fixture::new(); let d = draft();
        write_at(&f.path(), Some(&d)).unwrap();
        std::fs::write(f.path().with_extension("json.tmp"), b"half written").unwrap();
        assert_eq!(read_at(&f.path()).unwrap().unwrap().updated_at_ms, 123);
        std::fs::remove_file(f.path().with_extension("json.tmp")).unwrap();
        std::fs::create_dir(f.path().with_extension("json.tmp")).unwrap();
        assert!(write_at(&f.path(), None).is_err());
        assert!(read_at(&f.path()).unwrap().is_some());
        std::fs::remove_dir(f.path().with_extension("json.tmp")).unwrap();
        write_at(&f.path(), None).unwrap(); assert!(read_at(&f.path()).unwrap().is_none());
    }
    #[test]
    fn corrupt_and_future_drafts_are_preserved_until_explicit_discard() {
        let f = Fixture::new();
        for raw in ["invalid".to_string(), serde_json::to_string(&ProfileDraft { version: 2, ..draft() }).unwrap()] {
            std::fs::write(f.path(), &raw).unwrap();
            assert!(read_at(&f.path()).is_err());
            assert!(write_at(&f.path(), Some(&draft())).is_err());
            assert_eq!(std::fs::read_to_string(f.path()).unwrap(), raw);
            write_at(&f.path(), None).unwrap(); assert!(read_at(&f.path()).unwrap().is_none());
        }
    }
}
