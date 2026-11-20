//! Plays one local sound at a time, on its own volume, without touching the
//! system mixer. A new sound replaces the one that is playing.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;
use std::time::Duration;

const MAX_BYTES: u64 = 20 * 1024 * 1024;

enum Job {
    Play { path: PathBuf, volume: f32, reply: Sender<Result<(), String>> },
    Stop,
}

/// `vol:80|C:\sounds\horn.wav`, or a bare path at full volume.
pub(crate) fn parse_sound(param: &str) -> Result<(u8, PathBuf), String> {
    let param = param.trim();
    if param.is_empty() {
        return Err("Choose a sound file".into());
    }
    let (volume, raw_path) = if let Some(rest) = param.strip_prefix("vol:") {
        let (volume, path) = rest.split_once('|').ok_or("Sound needs a file path")?;
        let volume: u8 = volume.trim().parse().map_err(|_| "Volume must be 0 to 100".to_string())?;
        if volume > 100 {
            return Err("Volume must be 0 to 100".into());
        }
        (volume, path)
    } else {
        (100, param)
    };
    if raw_path.contains("://") {
        return Err("Use a file on this computer".into());
    }
    let path = PathBuf::from(raw_path.trim());
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
    if !matches!(ext.as_str(), "wav" | "mp3" | "ogg") {
        return Err("Use a wav, mp3, or ogg file".into());
    }
    let meta = std::fs::metadata(&path).map_err(|_| "That sound file was not found".to_string())?;
    if !meta.is_file() {
        return Err("That sound file was not found".into());
    }
    if meta.len() > MAX_BYTES {
        return Err("Sound files must be 20 MB or smaller".into());
    }
    Ok((volume, path))
}

pub(crate) fn play(param: &str) -> Result<String, String> {
    let (volume, path) = parse_sound(param)?;
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("sound").to_string();
    let (reply_tx, reply_rx) = mpsc::channel();
    player()
        .send(Job::Play { path, volume: volume as f32 / 100.0, reply: reply_tx })
        .map_err(|_| "The sound player is not running".to_string())?;
    match reply_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(())) => Ok(format!("Playing {name}")),
        Ok(Err(error)) => Err(error),
        Err(_) => Err("The sound player did not start in time".into()),
    }
}

pub(crate) fn stop() -> Result<String, String> {
    player().send(Job::Stop).map_err(|_| "The sound player is not running".to_string())?;
    Ok("Stopped the sound".into())
}

fn player() -> &'static Sender<Job> {
    static SENDER: OnceLock<Sender<Job>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new().name("deckforge-sound".into()).spawn(move || run_player(rx)).ok();
        tx
    })
}

fn run_player(rx: mpsc::Receiver<Job>) {
    let mut output: Option<(rodio::MixerDeviceSink, rodio::Player)> = None;
    while let Ok(job) = rx.recv() {
        match job {
            Job::Stop => {
                if let Some((_, player)) = &output {
                    player.stop();
                }
            }
            Job::Play { path, volume, reply } => {
                let result = start(&mut output, &path, volume);
                let _ = reply.send(result);
            }
        }
    }
}

fn start(output: &mut Option<(rodio::MixerDeviceSink, rodio::Player)>, path: &Path, volume: f32) -> Result<(), String> {
    if output.is_none() {
        let sink = rodio::DeviceSinkBuilder::open_default_sink().map_err(|error| format!("No audio output: {error}"))?;
        let player = rodio::Player::connect_new(sink.mixer());
        *output = Some((sink, player));
    }
    let (_, player) = output.as_ref().expect("opened above");
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let source = rodio::Decoder::try_from(file).map_err(|error| format!("Could not read that sound: {error}"))?;
    player.stop();
    player.append(source);
    player.set_volume(volume);
    player.play();
    Ok(())
}

/// Opens the system file dialog on its own thread. The dialog needs an STA apartment.
pub(crate) fn pick_file() -> Result<Option<String>, String> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("deckforge-sound-dialog".into())
        .spawn(move || {
            let _ = tx.send(pick_file_sta());
        })
        .map_err(|error| error.to_string())?;
    rx.recv().unwrap_or_else(|_| Err("The file dialog closed unexpectedly".into()))
}

fn pick_file_sta() -> Result<Option<String>, String> {
    use std::ffi::c_void;
    use windows::core::w;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, SIGDN_FILESYSPATH,
    };

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().map_err(|error| error.to_string())?;
        let outcome = (|| {
            let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).map_err(|error| error.to_string())?;
            dialog.SetOptions(FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM).map_err(|error| error.to_string())?;
            let filters = [
                COMDLG_FILTERSPEC { pszName: w!("Sound files"), pszSpec: w!("*.wav;*.mp3;*.ogg") },
                COMDLG_FILTERSPEC { pszName: w!("All files"), pszSpec: w!("*.*") },
            ];
            dialog.SetFileTypes(&filters).map_err(|error| error.to_string())?;
            dialog.SetTitle(w!("Choose a sound")).map_err(|error| error.to_string())?;
            match dialog.Show(None) {
                Ok(()) => {
                    let item = dialog.GetResult().map_err(|error| error.to_string())?;
                    let name = item.GetDisplayName(SIGDN_FILESYSPATH).map_err(|error| error.to_string())?;
                    let path = name.to_string().unwrap_or_default();
                    CoTaskMemFree(Some(name.0 as *const c_void));
                    if path.is_empty() {
                        Err("That sound file was not found".into())
                    } else {
                        Ok(Some(path))
                    }
                }
                Err(error) if error.code().0 == 0x8007_04C7u32 as i32 => Ok(None),
                Err(error) => Err(error.to_string()),
            }
        })();
        CoUninitialize();
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, b"RIFF").unwrap();
        path
    }

    #[test]
    fn a_bare_path_plays_at_full_volume() {
        let path = tiny("deckforge-sound-plain.wav");
        let (volume, got) = parse_sound(&path.display().to_string()).unwrap();
        assert_eq!(volume, 100);
        assert_eq!(got, path);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn volume_is_read_from_the_prefix() {
        let path = tiny("deckforge-sound-quiet.mp3");
        let param = format!("vol:40|{}", path.display());
        let (volume, got) = parse_sound(&param).unwrap();
        assert_eq!(volume, 40);
        assert_eq!(got, path);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn a_volume_past_100_is_rejected() {
        let path = tiny("deckforge-sound-loud.ogg");
        let error = parse_sound(&format!("vol:101|{}", path.display())).unwrap_err();
        assert!(error.contains("0 to 100"), "{error}");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn a_web_address_is_rejected() {
        let error = parse_sound("https://example.com/horn.wav").unwrap_err();
        assert!(error.contains("this computer"), "{error}");
    }

    #[test]
    fn a_missing_file_is_rejected() {
        let error = parse_sound(r"C:\no-such-deckforge-sound.wav").unwrap_err();
        assert!(error.contains("not found"), "{error}");
    }

    #[test]
    fn a_file_over_20_mb_is_rejected() {
        let path = std::env::temp_dir().join("deckforge-sound-huge.wav");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_BYTES + 1).unwrap();
        let error = parse_sound(&path.display().to_string()).unwrap_err();
        assert!(error.contains("20 MB"), "{error}");
        std::fs::remove_file(path).ok();
    }
}
