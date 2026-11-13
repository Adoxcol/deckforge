//! Installs plugins from `.streamDeckPlugin` files (a zip of one `.sdPlugin` folder) and
//! finds open-source plugins on GitHub.

use super::manifest::{parse, Manifest};
use serde::Serialize;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

const MAX_TOTAL_BYTES: u64 = 400 * 1024 * 1024;
const MAX_ENTRIES: usize = 20_000;
pub const MAX_DOWNLOAD_BYTES: usize = 200 * 1024 * 1024;

/// Unpacks a plugin archive into `root`, replacing an older copy of the same folder.
/// `before_replace` runs just before an existing folder is removed (to stop the plugin).
pub fn unpack(bytes: &[u8], root: &Path, before_replace: impl FnOnce(&Path)) -> Result<Manifest, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| "That file is not a plugin package.".to_string())?;
    if zip.len() > MAX_ENTRIES {
        return Err("The plugin package has too many files.".into());
    }
    // Every entry must sit inside one `<name>.sdPlugin` folder.
    let mut folder: Option<String> = None;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let path = entry.enclosed_name().ok_or("The plugin package has an unsafe file path.")?;
        let first = path.components().next().map(|c| c.as_os_str().to_string_lossy().to_string()).unwrap_or_default();
        if first.starts_with("__MACOSX") {
            continue;
        }
        if !first.to_ascii_lowercase().ends_with(".sdplugin") {
            return Err("The package does not contain a .sdPlugin folder.".into());
        }
        match &folder {
            None => folder = Some(first),
            Some(f) if f.eq_ignore_ascii_case(&first) => {}
            Some(_) => return Err("The package contains more than one plugin.".into()),
        }
        total = total.saturating_add(entry.size());
        if total > MAX_TOTAL_BYTES {
            return Err("The plugin package is too large.".into());
        }
    }
    let folder = folder.ok_or("The plugin package is empty.")?;

    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let staging = root.join(format!(".installing-{}", crate::drivers::hal::now_ms()));
    let result = (|| -> Result<(), String> {
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
            let Some(rel) = entry.enclosed_name() else { continue };
            if rel.components().next().map(|c| c.as_os_str().to_string_lossy().starts_with("__MACOSX")).unwrap_or(false) {
                continue;
            }
            let out = staging.join(rel);
            if entry.is_dir() {
                std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut data = Vec::with_capacity(entry.size().min(64 * 1024 * 1024) as usize);
            entry.by_ref().take(MAX_TOTAL_BYTES).read_to_end(&mut data).map_err(|e| e.to_string())?;
            std::fs::write(&out, data).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(err) = result {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(err);
    }
    let unpacked = staging.join(&folder);
    let manifest = std::fs::read_to_string(unpacked.join("manifest.json"))
        .ok()
        .and_then(|raw| parse(&root.join(&folder), &raw, "DeckForge"));
    let Some(manifest) = manifest else {
        let _ = std::fs::remove_dir_all(&staging);
        return Err("The plugin's manifest.json is missing or broken.".into());
    };
    let target = root.join(&folder);
    if target.exists() {
        before_replace(&target);
        if let Err(e) = remove_with_retry(&target) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(format!("Could not replace the old copy of this plugin: {e}"));
        }
    }
    std::fs::rename(&unpacked, &target).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&staging);
    Ok(manifest)
}

/// A plugin that just stopped may hold its files open for a moment.
fn remove_with_retry(path: &Path) -> std::io::Result<()> {
    let mut last = Ok(());
    for _ in 0..10 {
        last = std::fs::remove_dir_all(path);
        if last.is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    last
}

/// Picks the plugin file to install from one release's files.
fn pick_asset(assets: &[serde_json::Value]) -> Option<serde_json::Value> {
    let name = |a: &serde_json::Value| a.get("name").and_then(|v| v.as_str()).unwrap_or("").to_ascii_lowercase();
    let mut plugins: Vec<&serde_json::Value> = assets.iter().filter(|a| name(a).ends_with(".streamdeckplugin")).collect();
    let rank = |n: &str| {
        if n.contains("win") {
            0
        } else if n.contains("mac") || n.contains("osx") || n.contains("darwin") {
            2
        } else {
            1
        }
    };
    plugins.sort_by_key(|a| rank(&name(a)));
    plugins.first().map(|a| (*a).clone())
}

pub fn deckforge_root() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("DeckForge").join("plugins")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubPlugin {
    /// "owner/name".
    pub repo: String,
    pub name: String,
    pub description: String,
    pub stars: u64,
    pub url: String,
    pub owner_avatar: String,
    pub updated_at: String,
}

fn client() -> Result<reqwest::Client, String> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::Client::builder()
        .user_agent("DeckForge-plugin-browser")
        .connect_timeout(std::time::Duration::from_secs(20))
        // A download may take a while; only give up when it stalls.
        .read_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())
}

fn github_error(status: reqwest::StatusCode) -> String {
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        "GitHub is limiting searches for now. Try again in a minute.".into()
    } else {
        format!("GitHub answered {status}.")
    }
}

/// Repositories tagged as Stream Deck plugins, most starred first.
pub async fn search_github(query: &str) -> Result<Vec<GithubPlugin>, String> {
    let words: String = query
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .take(60)
        .collect();
    let q = format!("{} topic:streamdeck-plugin", words.trim());
    let resp = client()?
        // Only letters, digits, spaces, hyphens and the colon of `topic:` remain, so this is safe to put in a URL.
        .get(format!(
            "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page=40",
            q.trim().replace(' ', "+")
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Could not reach GitHub. Check your internet connection.".to_string())?;
    if !resp.status().is_success() {
        return Err(github_error(resp.status()));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = body.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    Ok(items
        .iter()
        .filter(|item| !item.get("archived").and_then(|v| v.as_bool()).unwrap_or(false))
        .map(|item| {
            let s = |k: &str| item.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            GithubPlugin {
                repo: s("full_name"),
                name: s("name"),
                description: s("description"),
                stars: item.get("stargazers_count").and_then(|v| v.as_u64()).unwrap_or(0),
                url: s("html_url"),
                owner_avatar: item.pointer("/owner/avatar_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                updated_at: s("pushed_at"),
            }
        })
        .collect())
}

/// Downloads the plugin package from a repository's newest release.
pub async fn download_latest(repo: &str) -> Result<Vec<u8>, String> {
    let valid = repo.split('/').count() == 2
        && repo.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
        && !repo.contains("..");
    if !valid {
        return Err("That is not a GitHub repository.".into());
    }
    let http = client()?;
    let resp = http
        .get(format!("https://api.github.com/repos/{repo}/releases?per_page=10"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Could not reach GitHub. Check your internet connection.".to_string())?;
    if !resp.status().is_success() {
        return Err(github_error(resp.status()));
    }
    let releases: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    // The newest release that has a plugin file; within it, a Windows build beats a Mac one.
    let asset = releases
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| !r.get("draft").and_then(|v| v.as_bool()).unwrap_or(false))
        .find_map(|r| {
            let assets = r.get("assets").and_then(|a| a.as_array()).cloned().unwrap_or_default();
            pick_asset(&assets)
        })
        .ok_or("This project has no ready-to-install plugin file in its releases. Open its page to see how to install it.")?;
    let size = asset.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
    if size as usize > MAX_DOWNLOAD_BYTES {
        return Err("The plugin file is too large.".into());
    }
    let url = asset.get("browser_download_url").and_then(|v| v.as_str()).ok_or("The release file has no download link.")?;
    if !url.starts_with("https://github.com/") {
        return Err("The release file is not hosted on GitHub.".into());
    }
    let bytes = http
        .get(url)
        .send()
        .await
        .map_err(|e| format!("The download failed: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| format!("The download failed: {e}"))?;
    if bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err("The plugin file is too large.".into());
    }
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut out);
            let opts = zip::write::SimpleFileOptions::default();
            for (name, data) in files {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        out.into_inner()
    }

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("deckforge-install-{name}-{}-{}", std::process::id(), crate::drivers::hal::now_ms()))
    }

    #[test]
    fn a_package_unpacks_and_replaces_an_older_copy() {
        let root = temp_root("ok");
        let v1 = archive(&[
            ("com.me.x.sdPlugin/manifest.json", br#"{"Name":"X","Version":"1","CodePath":"run.exe"}"#),
            ("com.me.x.sdPlugin/old.txt", b"old"),
            ("__MACOSX/._junk", b""),
        ]);
        let m = unpack(&v1, &root, |_| panic!("nothing to replace yet")).unwrap();
        assert_eq!(m.uuid, "com.me.x");
        assert_eq!(m.dir, root.join("com.me.x.sdPlugin"));
        assert!(root.join("com.me.x.sdPlugin/old.txt").is_file());

        let v2 = archive(&[("com.me.x.sdPlugin/manifest.json", br#"{"Name":"X","Version":"2"}"#)]);
        let mut replaced = false;
        let m = unpack(&v2, &root, |_| replaced = true).unwrap();
        assert!(replaced);
        assert_eq!(m.version, "2");
        assert!(!root.join("com.me.x.sdPlugin/old.txt").exists());
        let leftovers = std::fs::read_dir(&root).unwrap().count();
        assert_eq!(leftovers, 1, "staging folders are cleaned up");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn windows_packages_are_picked_over_mac_ones() {
        let assets = serde_json::json!([
            { "name": "x.mac.streamDeckPlugin" }, { "name": "readme.md" }, { "name": "x.win.streamDeckPlugin" }
        ]);
        let picked = pick_asset(assets.as_array().unwrap()).unwrap();
        assert_eq!(picked["name"], "x.win.streamDeckPlugin");
        assert!(pick_asset(&[serde_json::json!({ "name": "notes.txt" })]).is_none());
    }

    #[test]
    fn unsafe_or_odd_packages_are_refused() {
        let root = temp_root("bad");
        let traversal = archive(&[("../evil.sdPlugin/manifest.json", b"{}")]);
        assert!(unpack(&traversal, &root, |_| {}).is_err());
        let two = archive(&[("a.sdPlugin/manifest.json", b"{}"), ("b.sdPlugin/manifest.json", b"{}")]);
        assert!(unpack(&two, &root, |_| {}).unwrap_err().contains("more than one"));
        let loose = archive(&[("manifest.json", b"{}")]);
        assert!(unpack(&loose, &root, |_| {}).is_err());
        let no_manifest = archive(&[("a.sdPlugin/readme.txt", b"hi")]);
        assert!(unpack(&no_manifest, &root, |_| {}).unwrap_err().contains("manifest"));
        assert!(unpack(b"not a zip", &root, |_| {}).is_err());
        assert!(!root.join("a.sdPlugin").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    /// Run with `cargo test --lib github_live -- --ignored --nocapture`.
    #[test]
    #[ignore = "talks to GitHub"]
    fn github_live_search_and_download() {
        tauri::async_runtime::block_on(async {
            let found = search_github("").await.unwrap();
            println!("found {} repos; top: {:?}", found.len(), found.iter().take(5).map(|r| (&r.repo, r.stars)).collect::<Vec<_>>());
            assert!(!found.is_empty());
            let mut ok = 0;
            for repo in found.iter().take(12) {
                match download_latest(&repo.repo).await {
                    Ok(bytes) => {
                        let root = temp_root("live");
                        let m = unpack(&bytes, &root, |_| {});
                        println!("{}: {} bytes -> {:?}", repo.repo, bytes.len(), m.as_ref().map(|m| (&m.uuid, m.runtime)));
                        if m.is_ok() { ok += 1; break; }
                        let _ = std::fs::remove_dir_all(root);
                    }
                    Err(e) => println!("{}: {e}", repo.repo),
                }
            }
            assert!(ok > 0, "no installable plugin among the top results");
        });
    }
}
