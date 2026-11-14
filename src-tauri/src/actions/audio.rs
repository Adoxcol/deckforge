#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioFace {
    pub mic_muted: Option<bool>,
    pub speaker_muted: Option<bool>,
    pub volume_percent: Option<u8>,
}

pub fn read_audio_face() -> AudioFace {
    #[cfg(target_os = "windows")]
    {
        let _com = ComGuard::new();
        AudioFace {
            mic_muted: endpoint_muted(true).ok(),
            speaker_muted: endpoint_muted(false).ok(),
            volume_percent: endpoint_volume().ok(),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        AudioFace {
            mic_muted: None,
            speaker_muted: None,
            volume_percent: None,
        }
    }
}

pub fn toggle_microphone_mute() -> Result<String, String> {
    toggle_endpoint(true, "Microphone")
}

pub fn toggle_speaker_mute() -> Result<String, String> {
    toggle_endpoint(false, "Speakers")
}

fn toggle_endpoint(capture: bool, label: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let _com = ComGuard::new();
        let muted = endpoint_muted(capture)?;
        set_endpoint_mute(capture, !muted)?;
        Ok(if muted {
            format!("{label} live")
        } else {
            format!("{label} muted")
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (capture, label);
        Err("Audio mute is only available on Windows".to_string())
    }
}

#[cfg(target_os = "windows")]
struct ComGuard;

#[cfg(target_os = "windows")]
impl ComGuard {
    fn new() -> Self {
        unsafe {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        Self
    }
}

#[cfg(target_os = "windows")]
fn endpoint_volume() -> Result<u8, String> {
    let volume = endpoint(false)?;
    let scalar = unsafe { volume.GetMasterVolumeLevelScalar() }.map_err(|e| e.to_string())?;
    Ok((scalar.clamp(0.0, 1.0) * 100.0).round() as u8)
}

#[cfg(target_os = "windows")]
fn endpoint_muted(capture: bool) -> Result<bool, String> {
    let volume = endpoint(capture)?;
    let muted = unsafe { volume.GetMute() }.map_err(|e| e.to_string())?;
    Ok(muted.as_bool())
}

#[cfg(target_os = "windows")]
fn set_endpoint_mute(capture: bool, muted: bool) -> Result<(), String> {
    let volume = endpoint(capture)?;
    unsafe { volume.SetMute(muted, std::ptr::null()) }.map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn endpoint(capture: bool) -> Result<windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume, String> {
    use windows::Win32::Media::Audio::{
        eCapture, eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let flow = if capture { eCapture } else { eRender };
        let device = enumerator
            .GetDefaultAudioEndpoint(flow, eConsole)
            .map_err(|_| {
                if capture {
                    "Windows has no default microphone".to_string()
                } else {
                    "Windows has no default speakers".to_string()
                }
            })?;
        device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|e| e.to_string())
    }
}

/// A sound output Windows can play to.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOutput {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// The sound outputs that are plugged in and turned on.
pub fn list_outputs() -> Result<Vec<AudioOutput>, String> {
    #[cfg(target_os = "windows")]
    {
        let _com = ComGuard::new();
        outputs::list()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Sound outputs are only available on Windows".to_string())
    }
}

/// Picks which outputs a key moves between: names split by `|`, or every output when blank.
pub fn outputs_to_cycle<'a>(all: &'a [AudioOutput], param: &str) -> Vec<&'a AudioOutput> {
    let wanted: Vec<String> = param.split('|').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect();
    if wanted.is_empty() {
        return all.iter().collect();
    }
    wanted
        .iter()
        .filter_map(|w| {
            all.iter()
                .find(|o| o.name.to_lowercase() == *w)
                .or_else(|| all.iter().find(|o| o.name.to_lowercase().contains(w.as_str())))
        })
        .collect()
}

/// The output after the current default in `cycle`; the first one when the default is not in it.
pub fn next_output<'a>(cycle: &[&'a AudioOutput]) -> Option<&'a AudioOutput> {
    let at = cycle.iter().position(|o| o.is_default);
    match at {
        Some(i) => cycle.get((i + 1) % cycle.len()).copied(),
        None => cycle.first().copied(),
    }
}

/// Moves Windows' sound to the next output in the list (see `outputs_to_cycle`).
pub fn cycle_output(param: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let _com = ComGuard::new();
        let all = outputs::list()?;
        let cycle = outputs_to_cycle(&all, param);
        if cycle.is_empty() {
            return Err(format!("No sound output matches \"{param}\". Is it plugged in?"));
        }
        let next = next_output(&cycle).ok_or("No sound outputs")?;
        if next.is_default && cycle.len() == 1 {
            return Ok(format!("Sound already plays on {}", next.name));
        }
        outputs::set_default(&next.id)?;
        Ok(format!("Sound output: {}", next.name))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = param;
        Err("Sound outputs are only available on Windows".to_string())
    }
}

#[cfg(target_os = "windows")]
#[allow(non_snake_case)]
mod outputs {
    use super::AudioOutput;
    use std::ffi::c_void;
    use windows::core::{interface, GUID, HRESULT, IUnknown, IUnknown_Vtbl, PCWSTR};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::{
        eCommunications, eConsole, eMultimedia, eRender, ERole, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL, STGM_READ};

    /// Windows' own (undocumented but long-stable) interface for changing the default device,
    /// the one the Sound control panel uses.
    #[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
    unsafe trait IPolicyConfig: IUnknown {
        fn GetMixFormat(&self, id: PCWSTR, fmt: *mut *mut c_void) -> HRESULT;
        fn GetDeviceFormat(&self, id: PCWSTR, default: i32, fmt: *mut *mut c_void) -> HRESULT;
        fn ResetDeviceFormat(&self, id: PCWSTR) -> HRESULT;
        fn SetDeviceFormat(&self, id: PCWSTR, a: *mut c_void, b: *mut c_void) -> HRESULT;
        fn GetProcessingPeriod(&self, id: PCWSTR, default: i32, a: *mut i64, b: *mut i64) -> HRESULT;
        fn SetProcessingPeriod(&self, id: PCWSTR, a: *mut i64) -> HRESULT;
        fn GetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
        fn SetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
        fn GetPropertyValue(&self, id: PCWSTR, key: *const c_void, value: *mut c_void) -> HRESULT;
        fn SetPropertyValue(&self, id: PCWSTR, key: *const c_void, value: *mut c_void) -> HRESULT;
        fn SetDefaultEndpoint(&self, id: PCWSTR, role: ERole) -> HRESULT;
        fn SetEndpointVisibility(&self, id: PCWSTR, visible: i32) -> HRESULT;
    }

    const CPOLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

    pub fn list() -> Result<Vec<AudioOutput>, String> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let default_id = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .and_then(|d| d.GetId())
                .map(|p| {
                    let s = p.to_string().unwrap_or_default();
                    CoTaskMemFree(Some(p.0 as *const c_void));
                    s
                })
                .unwrap_or_default();
            let devices = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE).map_err(|e| e.to_string())?;
            let count = devices.GetCount().map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for i in 0..count {
                let Ok(device) = devices.Item(i) else { continue };
                let Ok(raw) = device.GetId() else { continue };
                let id = raw.to_string().unwrap_or_default();
                CoTaskMemFree(Some(raw.0 as *const c_void));
                let name = device
                    .OpenPropertyStore(STGM_READ)
                    .and_then(|store| store.GetValue(&PKEY_Device_FriendlyName))
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| id.clone());
                out.push(AudioOutput { is_default: id == default_id, id, name });
            }
            Ok(out)
        }
    }

    pub fn set_default(id: &str) -> Result<(), String> {
        let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let policy: IPolicyConfig =
                CoCreateInstance(&CPOLICY_CONFIG_CLIENT, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            for role in [eConsole, eMultimedia, eCommunications] {
                policy.SetDefaultEndpoint(PCWSTR(wide.as_ptr()), role).ok().map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod output_tests {
    use super::*;

    fn out(name: &str, is_default: bool) -> AudioOutput {
        AudioOutput { id: name.into(), name: name.into(), is_default }
    }

    #[test]
    fn a_key_moves_to_the_next_output_and_wraps() {
        let all = [out("Speakers (Realtek)", true), out("Headphones (USB)", false), out("TV (HDMI)", false)];
        let pair = outputs_to_cycle(&all, "headphones | speakers");
        assert_eq!(pair.len(), 2);
        assert_eq!(next_output(&pair).unwrap().name, "Headphones (USB)");
        assert_eq!(next_output(&outputs_to_cycle(&all, "")).unwrap().name, "Headphones (USB)");
        let wrap = [out("A", false), out("B", true)];
        assert_eq!(next_output(&outputs_to_cycle(&wrap, "")).unwrap().name, "A");
        assert_eq!(next_output(&outputs_to_cycle(&all, "tv")).unwrap().name, "TV (HDMI)");
        assert!(outputs_to_cycle(&all, "nothing").is_empty());
    }
}

#[cfg(test)]
mod output_live_tests {
    /// Lists this PC's outputs without changing anything. `cargo test --lib lists_real -- --ignored --nocapture`
    #[test]
    #[ignore = "reads this PC's sound devices"]
    fn lists_real_outputs() {
        let all = super::list_outputs().unwrap();
        println!("{all:#?}");
        assert!(!all.is_empty());
    }
}
