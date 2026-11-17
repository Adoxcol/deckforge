//! Optional profile to use on the first deck connection after process start.
use crate::db;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct Preference { profile_id: String }

fn path() -> PathBuf { db::profiles_store_path().with_file_name("startup-profile.json") }

pub(crate) fn load() -> Option<String> { load_at(&path()) }
fn load_at(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let preference: Preference = serde_json::from_str(&raw).ok()?;
    (!preference.profile_id.is_empty()).then_some(preference.profile_id)
}

pub(crate) fn save(id: Option<&str>) -> Result<(), String> { save_at(&path(), id) }
fn save_at(path: &Path, id: Option<&str>) -> Result<(), String> {
    if let Some(id) = id {
        if id.is_empty() { return Err("Choose a profile first.".into()); }
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        db::write_atomic(path, &serde_json::to_string(&Preference { profile_id: id.into() }).map_err(|e| e.to_string())?)
    } else {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn choice_survives_restart_and_can_return_to_last_applied() {
        let path = std::env::temp_dir().join(format!("deckforge-startup-{}.json", std::process::id()));
        save_at(&path, Some("profile-work")).unwrap();
        assert_eq!(load_at(&path).as_deref(), Some("profile-work"));
        save_at(&path, None).unwrap();
        assert_eq!(load_at(&path), None);
    }
}
