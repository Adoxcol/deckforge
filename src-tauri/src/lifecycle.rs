use tauri::{Emitter, Manager};

pub fn show_editor(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show(); let _ = window.unminimize(); let _ = window.set_focus();
    }
}

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder};
    let show = MenuItem::with_id(app, "show-editor", "Open DeckForge", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "request-quit", "Quit DeckForge…", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut tray = TrayIconBuilder::with_id("deckforge").menu(&menu).tooltip("DeckForge — running in background")
        .show_menu_on_left_click(true).on_menu_event(|app, event| {
            show_editor(app);
            if event.id.as_ref() == "request-quit" { let _ = app.emit("quit-requested", ()); }
        });
    if let Some(icon) = app.default_window_icon() { tray = tray.icon(icon.clone()); }
    tray.build(app)?;
    Ok(())
}

#[tauri::command]
pub async fn quit_application(app: tauri::AppHandle) -> Result<(), String> {
    let mode = crate::game_mode::status();
    if mode.active || mode.busy.is_some() {
        return Err("Turn off Game Mode before quitting so its services and power settings can be restored.".into());
    }
    crate::macro_run::cancel_running_macro();
    tauri::async_runtime::spawn_blocking(move || {
        for _ in 0..250 {
            if !crate::macro_run::macro_is_running() { crate::updates::install_on_quit(); app.exit(0); return Ok(()); }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        Err("A macro action is still finishing. Wait a moment and try quitting again.".into())
    }).await.map_err(|e| e.to_string())?
}

fn startup_command(exe: &std::path::Path) -> Result<String, String> {
    let path = exe.to_str().ok_or("Executable path is not valid Unicode")?;
    if path.contains('"') { return Err("Invalid executable path".into()); }
    Ok(format!("\"{path}\" --background"))
}

#[cfg(target_os = "windows")]
mod startup {
    use windows::{core::{w, PCWSTR}, Win32::{Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS}, System::Registry::*}};
    const KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    const NAME: PCWSTR = w!("DeckForge");

    pub fn enabled() -> Result<bool, String> {
        let mut data = vec![0u16; 32768]; let mut bytes = (data.len() * 2) as u32;
        let result = unsafe { RegGetValueW(HKEY_CURRENT_USER, KEY, NAME, RRF_RT_REG_SZ, None, Some(data.as_mut_ptr().cast()), Some(&mut bytes)) };
        if result == ERROR_FILE_NOT_FOUND { return Ok(false); }
        result.ok().map_err(|e| e.to_string())?;
        let end = data.iter().position(|c| *c == 0).unwrap_or(data.len());
        let expected = super::startup_command(&std::env::current_exe().map_err(|e| e.to_string())?)?;
        Ok(String::from_utf16_lossy(&data[..end]) == expected)
    }

    pub fn set(enabled: bool) -> Result<bool, String> {
        if enabled {
            let command = super::startup_command(&std::env::current_exe().map_err(|e| e.to_string())?)?;
            let data: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
            let mut key = HKEY::default();
            unsafe {
                RegCreateKeyExW(HKEY_CURRENT_USER, KEY, None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_SET_VALUE, None, &mut key, None)
                    .ok().map_err(|e| e.to_string())?;
                let result = RegSetKeyValueW(key, PCWSTR::null(), NAME, REG_SZ.0, Some(data.as_ptr().cast()), (data.len() * 2) as u32);
                let _ = RegCloseKey(key);
                result.ok().map_err(|e| e.to_string())?;
            }
        } else {
            let result = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, KEY, NAME) };
            if result != ERROR_FILE_NOT_FOUND && result != ERROR_SUCCESS { return Err(format!("Could not remove launch-at-sign-in entry (Windows error {})", result.0)); }
        }
        self::enabled()
    }
}

#[tauri::command]
pub fn get_launch_at_login() -> Result<bool, String> {
    #[cfg(target_os = "windows")] { startup::enabled() }
    #[cfg(not(target_os = "windows"))] { Ok(false) }
}

#[tauri::command]
pub fn set_launch_at_login(enabled: bool) -> Result<bool, String> {
    #[cfg(target_os = "windows")] { startup::set(enabled) }
    #[cfg(not(target_os = "windows"))] { let _ = enabled; Err("Launch at sign-in is supported on Windows only".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_quotes_paths_with_spaces_and_starts_in_background() {
        assert_eq!(startup_command(std::path::Path::new(r"C:\Program Files\DeckForge\deckforge.exe")).unwrap(),
            r#""C:\Program Files\DeckForge\deckforge.exe" --background"#);
        assert!(startup_command(std::path::Path::new("bad\"path.exe")).is_err());
    }
}
