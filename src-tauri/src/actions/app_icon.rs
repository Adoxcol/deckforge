//! Icons for apps named in auto-switch rules, read from the running app's .exe.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};

fn cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The app's icon as a PNG data URL, or None when the app can't be found or has no icon.
/// The .exe is taken from the running process, or else looked up in the usual install
/// folders so apps that are closed still show their icon. Found icons are cached.
pub fn app_icon_data_url(process_name: &str) -> Option<String> {
    let key = process_name.trim().to_ascii_lowercase();
    if key.is_empty() {
        return None;
    }
    if let Some(hit) = cache().lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return Some(hit.clone());
    }
    let exe = exe_path_for(&key).or_else(|| find_installed(&key))?;
    let png = icon_png(&exe)?;
    let url = format!("data:image/png;base64,{}", STANDARD.encode(png));
    cache().lock().unwrap_or_else(|e| e.into_inner()).insert(key, url.clone());
    Some(url)
}

fn exe_path_for(name: &str) -> Option<std::path::PathBuf> {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet)),
    );
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
    );
    sys.processes()
        .values()
        .find(|p| p.name().to_string_lossy().eq_ignore_ascii_case(name))
        .and_then(|p| p.exe().map(|e| e.to_path_buf()))
}

/// Looks for `name` (lower case) under Program Files and the per-user Programs folder, a few
/// levels deep: OBS keeps obs64.exe in `obs-studioin4bit`, VS Code installs per user.
fn find_installed(name: &str) -> Option<std::path::PathBuf> {
    let mut roots: Vec<std::path::PathBuf> = ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]
        .iter()
        .filter_map(|v| std::env::var_os(v).map(Into::into))
        .collect();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let local = std::path::PathBuf::from(local);
        roots.push(local.join("Programs"));
        roots.push(local);
    }
    if let Some(roaming) = std::env::var_os("APPDATA") {
        roots.push(roaming.into());
    }
    roots.dedup();
    roots.iter().find_map(|root| search(root, name, 4))
}

fn search(dir: &std::path::Path, name: &str, depth: u32) -> Option<std::path::PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
    for entry in &entries {
        if entry.file_name().to_string_lossy().eq_ignore_ascii_case(name) && entry.path().is_file() {
            return Some(entry.path());
        }
    }
    if depth == 0 {
        return None;
    }
    entries
        .iter()
        .filter(|e| e.file_type().map(|t| t.is_dir() && !t.is_symlink()).unwrap_or(false))
        .find_map(|e| search(&e.path(), name, depth - 1))
}

#[cfg(target_os = "windows")]
fn icon_png(exe: &std::path::Path) -> Option<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::UI::Shell::ExtractIconExW;
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    let wide: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        let mut large = HICON::default();
        if ExtractIconExW(PCWSTR(wide.as_ptr()), 0, Some(&mut large), None, 1) == 0 || large.is_invalid() {
            return None;
        }
        let mut info = ICONINFO::default();
        let ok = GetIconInfo(large, &mut info).is_ok();
        let result = if ok {
            let mut bm = BITMAP::default();
            GetObjectW(info.hbmColor.into(), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _));
            let (w, h) = (bm.bmWidth, bm.bmHeight);
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bgra = vec![0u8; (w.max(0) * h.max(0) * 4) as usize];
            let dc = CreateCompatibleDC(None);
            let lines = GetDIBits(dc, info.hbmColor, 0, h as u32, Some(bgra.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);
            let _ = DeleteDC(dc);
            if lines > 0 && w > 0 && h > 0 {
                encode_png(&mut bgra, w as u32, h as u32)
            } else {
                None
            }
        } else {
            None
        };
        if ok {
            let _ = DeleteObject(info.hbmColor.into());
            let _ = DeleteObject(info.hbmMask.into());
        }
        let _ = DestroyIcon(large);
        result
    }
}

#[cfg(not(target_os = "windows"))]
fn icon_png(_exe: &std::path::Path) -> Option<Vec<u8>> {
    None
}

fn encode_png(bgra: &mut [u8], w: u32, h: u32) -> Option<Vec<u8>> {
    // Old icons carry no alpha channel at all; treat them as opaque.
    let has_alpha = bgra.chunks_exact(4).any(|p| p[3] != 0);
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
        if !has_alpha {
            px[3] = 255;
        }
    }
    let img = image::RgbaImage::from_raw(w, h, bgra.to_vec())?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}
