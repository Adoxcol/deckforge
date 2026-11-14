//! Downloads a new release in the background and installs it when DeckForge quits.

use std::sync::Mutex;
use std::time::Duration;
use tauri::Emitter;
use tauri_plugin_updater::UpdaterExt;

/// A downloaded, signature-checked installer waiting for DeckForge to quit.
struct Ready {
    version: String,
    bytes: Vec<u8>,
}

static READY: Mutex<Option<Ready>> = Mutex::new(None);

/// The version waiting to be installed, if any.
#[tauri::command]
pub fn update_ready() -> Option<String> {
    READY.lock().ok()?.as_ref().map(|r| r.version.clone())
}

/// Checks soon after start and then every six hours. Dev builds never update.
pub fn start(app: tauri::AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio_sleep(Duration::from_secs(20)).await;
        loop {
            if update_ready().is_none() {
                if let Err(e) = fetch(&app).await {
                    eprintln!("update check failed: {e}");
                }
            }
            tokio_sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

async fn tokio_sleep(d: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}

async fn fetch(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(update) = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())? else {
        return Ok(());
    };
    // `download` checks the release signature before handing the bytes back.
    let bytes = update.download(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    let version = update.version.clone();
    *READY.lock().map_err(|e| e.to_string())? = Some(Ready { version: version.clone(), bytes });
    let _ = app.emit("update-ready", version);
    Ok(())
}

/// Starts the waiting installer silently, so it replaces DeckForge once it has quit.
/// Called right before the app exits; does nothing when no update is waiting.
pub fn install_on_quit() {
    let Some(ready) = READY.lock().ok().and_then(|mut r| r.take()) else { return };
    let path = std::env::temp_dir().join(format!("DeckForge_{}_update-setup.exe", ready.version));
    if std::fs::write(&path, &ready.bytes).is_err() {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        let _ = std::process::Command::new(&path).arg("/S").creation_flags(DETACHED_PROCESS).spawn();
    }
}
