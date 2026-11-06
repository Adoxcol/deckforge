#[derive(Debug, Clone)]
pub struct ForegroundWindow {
    pub exe_name: String,
    pub title: String,
}

pub fn foreground_window() -> Option<ForegroundWindow> {
    #[cfg(target_os = "windows")]
    {
        read_foreground()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[cfg(target_os = "windows")]
fn read_foreground() -> Option<ForegroundWindow> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };
    use windows::core::PWSTR;

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut title = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title);
        let title = String::from_utf16_lossy(&title[..title_len.max(0) as usize]);

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 520];
        let mut size = buf.len() as u32;
        let queried = QueryFullProcessImageNameW(handle, Default::default(), PWSTR(buf.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);
        queried.ok()?;
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        let exe_name = path
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(&path)
            .to_string();
        if exe_name.is_empty() {
            return None;
        }
        Some(ForegroundWindow { exe_name, title })
    }
}
