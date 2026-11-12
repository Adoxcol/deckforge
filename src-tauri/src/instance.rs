//! Hold a session-scoped OS mutex before loading state or spawning device workers.
#[cfg(target_os = "windows")]
mod native {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, GetLastError, SetLastError, HANDLE, WIN32_ERROR, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE};

    pub struct InstanceGuard(HANDLE);

    impl Drop for InstanceGuard {
        fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } }
    }

    fn acquire_named(name: PCWSTR) -> Result<Option<InstanceGuard>, String> {
        unsafe {
            SetLastError(WIN32_ERROR(0));
            let handle = CreateMutexW(None, false, name).map_err(|e| e.to_string())?;
            let existed = GetLastError() == ERROR_ALREADY_EXISTS;
            let guard = InstanceGuard(handle);
            if existed { drop(guard); Ok(None) } else { Ok(Some(guard)) }
        }
    }

    pub fn acquire() -> Result<Option<InstanceGuard>, String> {
        acquire_named(w!("Local\\org.deckforge.studio.instance"))
    }

    pub fn focus_existing() {
        unsafe {
            if let Ok(window) = FindWindowW(PCWSTR::null(), w!("DeckForge — Universal Open-Source Stream Deck Controller")) {
                let _ = ShowWindow(window, SW_RESTORE);
                let _ = SetForegroundWindow(window);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn second_instance_is_rejected_and_release_allows_a_new_instance() {
            let name: Vec<u16> = format!("Local\\deckforge-test-instance-{}", std::process::id())
                .encode_utf16().chain(Some(0)).collect();
            let first = acquire_named(PCWSTR(name.as_ptr())).unwrap().unwrap();
            assert!(acquire_named(PCWSTR(name.as_ptr())).unwrap().is_none());
            drop(first);
            assert!(acquire_named(PCWSTR(name.as_ptr())).unwrap().is_some());
        }
    }
}

#[cfg(target_os = "windows")]
pub use native::{acquire, focus_existing};

#[cfg(not(target_os = "windows"))]
pub fn acquire() -> Result<Option<()>, String> { Ok(Some(())) }
#[cfg(not(target_os = "windows"))]
pub fn focus_existing() {}
