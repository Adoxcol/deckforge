//! App volume targets use executable names, never saved process IDs.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioApp { pub exe: String, pub sessions: usize }

fn target(raw: &str) -> Result<String, String> {
    let name = raw.trim().to_lowercase();
    if !name.ends_with(".exe") || name.len() <= 4 || name.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')) {
        return Err("Choose an audio app or enter its executable name, such as spotify.exe".into());
    }
    Ok(name)
}
fn adjusted(volume: f32, up: bool) -> f32 { (volume + if up { 0.02 } else { -0.02 }).clamp(0.0, 1.0) }
fn mute_target(muted: impl Iterator<Item = bool>) -> bool { !muted.fold(true, |all, value| all && value) }

pub fn list() -> Result<Vec<AudioApp>, String> {
    #[cfg(target_os = "windows")]
    {
        let _com = native::Com::new()?;
        let mut apps = std::collections::BTreeMap::<String, usize>::new();
        for session in native::sessions()? { *apps.entry(session.exe).or_default() += 1; }
        Ok(apps.into_iter().map(|(exe, sessions)| AudioApp { exe, sessions }).collect())
    }
    #[cfg(not(target_os = "windows"))]
    { Err("App audio controls require Windows".into()) }
}

pub fn execute(action: &str, param: &str) -> Result<String, String> {
    let exe = target(param)?;
    if !matches!(action, "audio.app_volume_up" | "audio.app_volume_down" | "audio.app_mute") {
        return Err("Unknown app audio action".into());
    }
    #[cfg(target_os = "windows")]
    {
        let _com = native::Com::new()?;
        let sessions: Vec<_> = native::sessions()?.into_iter().filter(|s| s.exe == exe).collect();
        if sessions.is_empty() { return Err(format!("No audio session for {exe}. Open the app and play audio first.")); }
        // Read every state before writing, and use one mute target for all of an
        // app's sessions, including sessions on different output devices.
        let states = sessions.iter().map(|s| unsafe {
            Ok((s.volume.GetMasterVolume().map_err(|e| e.to_string())?, s.volume.GetMute().map_err(|e| e.to_string())?.as_bool()))
        }).collect::<Result<Vec<_>, String>>()?;
        let muted = mute_target(states.iter().map(|(_, mute)| *mute));
        let mut failed = 0;
        for (session, (volume, _)) in sessions.iter().zip(states) {
            let result = unsafe {
                if action == "audio.app_mute" { session.volume.SetMute(muted, std::ptr::null()) }
                else { session.volume.SetMasterVolume(adjusted(volume, action == "audio.app_volume_up"), std::ptr::null()) }
            };
            if result.is_err() { failed += 1; }
        }
        if failed > 0 { return Err(format!("Updated {} of {} audio sessions for {exe}; {failed} failed. The app or output may have changed.", sessions.len() - failed, sessions.len())); }
        let change = match action { "audio.app_mute" => if muted { "muted" } else { "unmuted" }, "audio.app_volume_up" => "volume increased", _ => "volume decreased" };
        Ok(format!("{exe}: {change} ({} sessions)", sessions.len()))
    }
    #[cfg(not(target_os = "windows"))]
    { let _ = exe; Err("App audio controls require Windows".into()) }
}

#[cfg(target_os = "windows")]
mod native {
    use windows::core::Interface;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;
    pub(super) struct Com;
    impl Com {
        pub(super) fn new() -> Result<Self, String> {
            unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().map_err(|e| e.to_string())?; }
            Ok(Self)
        }
    }
    impl Drop for Com { fn drop(&mut self) { unsafe { CoUninitialize(); } } }
    pub(super) struct Session { pub exe: String, pub volume: ISimpleAudioVolume }
    fn process_name(pid: u32) -> Option<String> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_NAME_WIN32};
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut path = vec![0u16; 32768];
            let mut len = path.len() as u32;
            let result = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, windows::core::PWSTR(path.as_mut_ptr()), &mut len);
            let _ = CloseHandle(handle);
            result.ok()?;
            String::from_utf16_lossy(&path[..len as usize]).rsplit('\\').next().map(str::to_lowercase)
        }
    }
    pub(super) fn sessions() -> Result<Vec<Session>, String> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let endpoints = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE).map_err(|e| e.to_string())?;
            let mut result = Vec::new();
            let mut names = std::collections::HashMap::new();
            for index in 0..endpoints.GetCount().map_err(|e| e.to_string())? {
                let device = endpoints.Item(index).map_err(|e| e.to_string())?;
                let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
                let sessions = manager.GetSessionEnumerator().map_err(|e| e.to_string())?;
                for index in 0..sessions.GetCount().map_err(|e| e.to_string())? {
                    let Ok(control) = sessions.GetSession(index) else { continue; };
                    if control.GetState().ok() == Some(AudioSessionStateExpired) { continue; }
                    let Ok(control2) = control.cast::<IAudioSessionControl2>() else { continue; };
                    let Ok(pid) = control2.GetProcessId() else { continue; };
                    if pid == 0 { continue; } // System sounds are not an application.
                    let name = names.entry(pid).or_insert_with(|| process_name(pid));
                    let Some(exe) = name.clone() else { continue; };
                    let Ok(volume) = control.cast::<ISimpleAudioVolume>() else { continue; };
                    result.push(Session { exe, volume });
                }
            }
            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn targets_are_exact_executable_names_not_paths_or_wildcards() {
        assert_eq!(target(" Spotify.EXE ").unwrap(), "spotify.exe");
        for name in ["", "spotify", ".exe", "*.exe", "C:\\Apps\\spotify.exe", "foo/bar.exe"] { assert!(target(name).is_err()); }
    }
    #[test]
    fn volume_is_clamped_and_mixed_sessions_mute_together() {
        assert_eq!(adjusted(0.99, true), 1.0);
        assert_eq!(adjusted(0.01, false), 0.0);
        assert!((adjusted(0.5, true) - 0.52).abs() < 0.00001);
        assert!(mute_target([true, false].into_iter()));
        assert!(!mute_target([true, true].into_iter()));
    }
    #[test]
    #[ignore = "reads this PC's audio sessions; does not change volume"]
    fn lists_real_audio_apps() { let apps = list().unwrap(); assert!(apps.iter().all(|app| app.sessions > 0 && !app.exe.is_empty())); }
}
