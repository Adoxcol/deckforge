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
