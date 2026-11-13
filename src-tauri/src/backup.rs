use super::*;
use std::collections::BTreeSet;

const MAX_BACKUP_BYTES: usize = 96 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileBackup {
    kind: String,
    version: u32,
    profiles: Vec<Profile>,
    macros: Vec<MacroSequence>,
    media: Vec<media::MediaBackup>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredBackup {
    profiles: Vec<Profile>,
    macros: Vec<MacroSequence>,
    media_library: Vec<MediaPreview>,
}

fn media_ids(profiles: &[Profile]) -> BTreeSet<String> {
    profiles.iter().flat_map(|p| &p.pages).flat_map(|p| {
        p.keys.iter().filter_map(|k| k.media.as_ref())
            .chain(p.touch_zones.iter().filter_map(|z| z.media.as_ref()))
            .chain(p.strip_media.as_ref()).map(|m| m.media_id.clone())
    }).collect()
}

fn validate(backup: &ProfileBackup) -> Result<(), String> {
    if backup.kind != "deckforge.profiles-backup" || backup.version != 1 {
        return Err("Unsupported backup format or version".into());
    }
    if backup.profiles.is_empty() || backup.profiles.len() > 256 || backup.macros.len() > 1024 || backup.media.len() > 256 {
        return Err("Backup exceeds profile, macro or media limits, or contains no profiles".into());
    }
    let mut ids = BTreeSet::new();
    for p in &backup.profiles {
        if p.id.is_empty() || !ids.insert(&p.id) || p.pages.is_empty() || p.pages.len() > 256 {
            return Err("Invalid or duplicate profile ID, or invalid page count".into());
        }
        let mut pages = BTreeSet::new();
        for page in &p.pages {
            if page.id.is_empty() || !pages.insert(&page.id) || page.keys.is_empty() || page.keys.len() > 128
                || page.touch_zones.len() > 128 || page.encoders.len() > 128 {
                return Err("Invalid page ID or control count".into());
            }
        }
    }
    let mut macros = BTreeSet::new();
    for m in &backup.macros {
        if m.id.is_empty() || !macros.insert(&m.id) || m.steps.len() > 256 {
            return Err("Invalid macro ID or step count".into());
        }
    }
    let assets: BTreeSet<_> = backup.media.iter().map(|m| m.id.clone()).collect();
    if assets.len() != backup.media.len() || assets != media_ids(&backup.profiles) {
        return Err("Backup media does not match its profile references".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn export_profiles_backup(profiles: Vec<Profile>, macros: Vec<MacroSequence>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || export(profiles, macros)).await.map_err(|e| e.to_string())?
}

fn export(profiles: Vec<Profile>, macros: Vec<MacroSequence>) -> Result<String, String> {
    let media = MediaStore::default().backup(&media_ids(&profiles))?;
    let backup = ProfileBackup { kind: "deckforge.profiles-backup".into(), version: 1, profiles, macros, media };
    validate(&backup)?;
    let contents = serde_json::to_string(&backup).map_err(|e| e.to_string())?;
    if contents.len() > MAX_BACKUP_BYTES { return Err("Backup exceeds the 96 MB file limit".into()); }
    commands::save_export_file(format!("deckforge-backup-{}.json", now_ms()), contents)
}

fn restore(contents: &str, store: &mut MediaStore) -> Result<RestoredBackup, String> {
    if contents.len() > MAX_BACKUP_BYTES { return Err("Backup exceeds the 96 MB file limit".into()); }
    let mut backup: ProfileBackup = serde_json::from_str(contents).map_err(|e| format!("Invalid backup: {e}"))?;
    validate(&backup)?;
    let (ids, media_library) = store.restore_backup(&backup.media)?;
    for page in backup.profiles.iter_mut().flat_map(|p| &mut p.pages) {
        for media in page.keys.iter_mut().filter_map(|k| k.media.as_mut())
            .chain(page.touch_zones.iter_mut().filter_map(|z| z.media.as_mut()))
            .chain(page.strip_media.as_mut()) {
            media.media_id = ids[&media.media_id].clone();
        }
    }
    Ok(RestoredBackup { profiles: backup.profiles, macros: backup.macros, media_library })
}

/// Stages new media only; profiles are returned as editor drafts and never applied here.
#[tauri::command]
pub async fn restore_profiles_backup(contents: String, state: State<'_, SharedState>) -> Result<RestoredBackup, String> {
    let shared = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        // Decoding and file writes must not hold the device worker's shared-state lock.
        let mut staged = MediaStore::default();
        let restored = restore(&contents, &mut staged)?;
        shared.lock_safe().media.merge_restored(staged);
        Ok(restored)
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProfileBackup {
        ProfileBackup { kind: "deckforge.profiles-backup".into(), version: 1,
            profiles: db::default_profiles(), macros: vec![], media: vec![] }
    }

    #[test]
    fn full_backup_round_trip_remaps_key_zone_and_strip_media() {
        let root = std::env::temp_dir().join(format!("deckforge-backup-test-{}-{}", std::process::id(), now_ms()));
        let source_dir = root.join("source");
        let target_dir = root.join("target");
        let mut source = MediaStore::load_from(source_dir.clone());
        let image = image::DynamicImage::new_rgb8(4, 4);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        source.stage_bytes("shared-image", "picture.png", bytes.get_ref()).unwrap();
        let mut backup = sample();
        let config = db::ScreenMediaConfig { media_id: "shared-image".into(), fit: "contain".into(), loop_playback: false, speed: 1.5 };
        let page = &mut backup.profiles[0].pages[0];
        page.keys[0].media = Some(config.clone());
        page.touch_zones[0].media = Some(config.clone());
        page.strip_media = Some(config);
        backup.media = source.backup(&media_ids(&backup.profiles)).unwrap();
        assert_eq!(backup.media.len(), 1); // Shared references are bundled once.
        let mut target = MediaStore::load_from(target_dir.clone());
        let restored = restore(&serde_json::to_string(&backup).unwrap(), &mut target).unwrap();
        let page = &restored.profiles[0].pages[0];
        let key = page.keys[0].media.as_ref().unwrap();
        assert_ne!(key.media_id, "shared-image");
        assert_eq!(page.touch_zones[0].media.as_ref().unwrap().media_id, key.media_id);
        assert_eq!(page.strip_media.as_ref().unwrap().media_id, key.media_id);
        assert_eq!(key.fit, "contain"); assert!(!key.loop_playback); assert_eq!(key.speed, 1.5);
        assert_eq!(MediaStore::load_from(target_dir.clone()).previews().len(), 1);
        for dir in [source_dir, target_dir] {
            for entry in std::fs::read_dir(&dir).unwrap() { std::fs::remove_file(entry.unwrap().path()).unwrap(); }
            std::fs::remove_dir(dir).unwrap();
        }
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn backup_rejects_unknown_versions_duplicates_and_missing_media() {
        let mut backup = sample();
        assert!(validate(&backup).is_ok());
        backup.version = 2;
        assert!(validate(&backup).is_err());
        backup.version = 1;
        backup.profiles.push(backup.profiles[0].clone());
        assert!(validate(&backup).is_err());
        backup.profiles.pop();
        backup.profiles[0].pages[0].keys[0].media = Some(db::ScreenMediaConfig {
            media_id: "missing".into(), fit: "cover".into(), loop_playback: true, speed: 1.0,
        });
        assert!(validate(&backup).is_err());
    }
}
