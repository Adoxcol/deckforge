use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsSettings {
    pub url: String,
    pub password: String,
}

impl Default for ObsSettings {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:4455".to_string(),
            password: String::new(),
        }
    }
}

impl ObsSettings {
    pub fn load() -> Self {
        let path = obs_store_path();
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = obs_store_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let raw = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        crate::db::write_atomic(&path, &raw)
    }
}

fn obs_store_path() -> std::path::PathBuf {
    let base = std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("DeckForge").join("obs.json")
}

/// OBS WebSocket v5 authentication string.
pub fn obs_authentication(password: &str, salt: &str, challenge: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    let secret = STANDARD.encode(hasher.finalize());
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(challenge.as_bytes());
    STANDARD.encode(hasher.finalize())
}

pub fn parse_obs_url(url: &str) -> Result<(String, u16), String> {
    let raw = url.trim();
    let raw = raw.strip_prefix("ws://").or_else(|| raw.strip_prefix("wss://")).unwrap_or(raw);
    let hostport = raw.split('/').next().unwrap_or(raw);
    if hostport.is_empty() {
        return Err("OBS address is empty. Use ws://127.0.0.1:4455.".to_string());
    }
    if let Some((host, port)) = hostport.rsplit_once(':') {
        let port = port.parse::<u16>().map_err(|_| format!("OBS port `{port}` is not a number"))?;
        if host.is_empty() {
            return Err("OBS address is missing a host.".to_string());
        }
        Ok((host.to_string(), port))
    } else {
        Ok((hostport.to_string(), 4455))
    }
}

/// Runs one OBS action. `param` carries the scene name or audio source for the actions that take one.
pub fn run_obs_action(settings: &ObsSettings, action_id: &str, param: &str) -> Result<String, String> {
    let param = param.trim();
    match action_id {
        "obs.toggle_virtualcam" => obs_request(settings, "ToggleVirtualCam", None).map(|_| "OBS virtual camera toggled".to_string()),
        "obs.toggle_replay" => obs_request(settings, "ToggleReplayBuffer", None).map(|_| "OBS replay buffer toggled".to_string()),
        "obs.pause_record" => obs_request(settings, "ToggleRecordPause", None).map(|_| "OBS recording pause toggled".to_string()),
        "obs.studio_transition" => obs_request(settings, "TriggerStudioModeTransition", None).map(|_| "OBS studio transition triggered".to_string()),
        "obs.set_scene" => {
            if param.is_empty() {
                return Err("pick a scene name in the key settings".to_string());
            }
            let mut data = serde_json::Map::new();
            data.insert("sceneName".to_string(), serde_json::Value::String(param.to_string()));
            obs_request(settings, "SetCurrentProgramScene", Some(serde_json::Value::Object(data)))?;
            Ok(format!("OBS scene set to {param}"))
        }
        "obs.toggle_mute" => {
            if param.is_empty() {
                return Err("type the OBS audio source name in the key settings".to_string());
            }
            let mut data = serde_json::Map::new();
            data.insert("inputName".to_string(), serde_json::Value::String(param.to_string()));
            obs_request(settings, "ToggleInputMute", Some(serde_json::Value::Object(data)))?;
            Ok(format!("OBS mute toggled for {param}"))
        }
        "obs.start_stream" => obs_request(settings, "ToggleStream", None).map(|_| "OBS stream toggled".to_string()),
        "obs.toggle_record" => obs_request(settings, "ToggleRecord", None).map(|_| "OBS recording toggled".to_string()),
        "obs.save_replay" => obs_request(settings, "SaveReplayBuffer", None).map(|_| "OBS saved the replay buffer".to_string()),
        "obs.next_scene" => switch_scene(settings, 1),
        "obs.prev_scene" => switch_scene(settings, -1),
        _ => Err(format!("OBS action `{action_id}` is not available")),
    }
}

fn switch_scene(settings: &ObsSettings, step: i32) -> Result<String, String> {
    let listed = obs_request(settings, "GetSceneList", None)?;
    let scenes = listed["responseData"]["scenes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let names: Vec<String> = scenes
        .iter()
        .filter_map(|scene| {
            scene["sceneName"]
                .as_str()
                .or_else(|| scene["scene_name"].as_str())
                .map(|s| s.to_string())
        })
        .collect();
    if names.is_empty() {
        return Err("OBS has no scenes".to_string());
    }
    let current = listed["responseData"]["currentProgramSceneName"]
        .as_str()
        .unwrap_or("");
    let index = names.iter().position(|name| name == current).unwrap_or(0) as i32;
    let next = (index + step).rem_euclid(names.len() as i32) as usize;
    let scene_name = names[next].clone();
    let mut data = serde_json::Map::new();
    data.insert("sceneName".to_string(), serde_json::Value::String(scene_name.clone()));
    obs_request(settings, "SetCurrentProgramScene", Some(serde_json::Value::Object(data)))?;
    Ok(format!("OBS scene set to {scene_name}"))
}

fn obs_request(
    settings: &ObsSettings,
    request_type: &str,
    request_data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let (host, port) = parse_obs_url(if settings.url.trim().is_empty() {
        "ws://127.0.0.1:4455"
    } else {
        settings.url.trim()
    })?;
    let addr = format!("{host}:{port}")
        .to_socket_addrs()
        .map_err(|e| format!("OBS address {host}:{port} failed: {e}"))?
        .next()
        .ok_or_else(|| format!("OBS address {host}:{port} did not resolve"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(900)).map_err(|_| {
        format!("OBS is not accepting WebSocket connections on {host}:{port}. In OBS, open Tools → WebSocket Server Settings and enable the server.")
    })?;
    stream.set_read_timeout(Some(Duration::from_millis(1500))).ok();
    stream.set_write_timeout(Some(Duration::from_millis(900))).ok();

    let key = STANDARD.encode(b"deckforge-obs-key!");
    let upgrade = format!(
        "GET / HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    stream.write_all(upgrade.as_bytes()).map_err(|e| e.to_string())?;
    read_http_upgrade(&mut stream)?;

    let hello = read_ws_text(&mut stream)?;
    let hello: serde_json::Value = serde_json::from_str(&hello).map_err(|e| format!("OBS hello was not JSON: {e}"))?;
    if hello["op"].as_u64() != Some(0) {
        return Err("OBS did not send a WebSocket hello".to_string());
    }
    let mut identify = serde_json::json!({
        "op": 1,
        "d": { "rpcVersion": 1, "eventSubscriptions": 0 }
    });
    if let (Some(challenge), Some(salt)) = (
        hello["d"]["authentication"]["challenge"].as_str(),
        hello["d"]["authentication"]["salt"].as_str(),
    ) {
        if settings.password.is_empty() {
            return Err("OBS WebSocket is asking for a password. Enter it in the button inspector.".to_string());
        }
        let auth = obs_authentication(&settings.password, salt, challenge);
        identify["d"]["authentication"] = serde_json::Value::String(auth);
    }
    write_ws_text(&mut stream, &identify.to_string())?;

    let request_id = format!("df-{request_type}");
    let mut request = serde_json::json!({
        "op": 6,
        "d": { "requestType": request_type, "requestId": request_id }
    });
    if let Some(data) = request_data {
        request["d"]["requestData"] = data;
    }
    write_ws_text(&mut stream, &request.to_string())?;

    for _ in 0..8 {
        let raw = read_ws_text(&mut stream)?;
        let value: serde_json::Value = serde_json::from_str(&raw).map_err(|e| format!("OBS reply was not JSON: {e}"))?;
        let op = value["op"].as_u64();
        if op == Some(2) {
            continue;
        }
        if op == Some(7) && value["d"]["requestId"].as_str() == Some(request_id.as_str()) {
            let ok = value["d"]["requestStatus"]["result"].as_bool().unwrap_or(false);
            if !ok {
                let comment = value["d"]["requestStatus"]["comment"].as_str().unwrap_or("OBS refused the request");
                return Err(comment.to_string());
            }
            return Ok(value["d"].clone());
        }
    }
    Err("OBS did not answer in time".to_string())
}

fn read_http_upgrade(stream: &mut TcpStream) -> Result<(), String> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() < 4096 {
        stream.read_exact(&mut byte).map_err(|_| "OBS closed the connection during the WebSocket handshake".to_string())?;
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            let text = String::from_utf8_lossy(&buf);
            if text.contains("101") {
                return Ok(());
            }
            return Err("OBS did not accept the WebSocket upgrade. Check the server port.".to_string());
        }
    }
    Err("OBS handshake was too long".to_string())
}

fn write_ws_text(stream: &mut TcpStream, text: &str) -> Result<(), String> {
    let payload = text.as_bytes();
    let mask = [0x44, 0x66, 0x31, 0x09];
    let mut frame = Vec::with_capacity(payload.len() + 8);
    frame.push(0x81);
    if payload.len() < 126 {
        frame.push(0x80 | payload.len() as u8);
    } else if payload.len() < 65536 {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        return Err("OBS message is too large".to_string());
    }
    frame.extend_from_slice(&mask);
    for (i, byte) in payload.iter().enumerate() {
        frame.push(byte ^ mask[i % 4]);
    }
    stream.write_all(&frame).map_err(|e| e.to_string())
}

fn read_ws_text(stream: &mut TcpStream) -> Result<String, String> {
    loop {
        let mut header = [0u8; 2];
        stream.read_exact(&mut header).map_err(|_| "OBS closed the connection".to_string())?;
        let opcode = header[0] & 0x0f;
        let masked = header[1] & 0x80 != 0;
        let mut len = (header[1] & 0x7f) as usize;
        if len == 126 {
            let mut extra = [0u8; 2];
            stream.read_exact(&mut extra).map_err(|e| e.to_string())?;
            len = u16::from_be_bytes(extra) as usize;
        } else if len == 127 {
            return Err("OBS sent a frame that is too large".to_string());
        }
        let mut mask = [0u8; 4];
        if masked {
            stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
        }
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).map_err(|_| "OBS closed the connection".to_string())?;
        if masked {
            for (i, byte) in data.iter_mut().enumerate() {
                *byte ^= mask[i % 4];
            }
        }
        match opcode {
            0x1 => return String::from_utf8(data).map_err(|e| e.to_string()),
            0x8 => return Err("OBS closed the WebSocket".to_string()),
            0x9 => {
                let mut pong = vec![0x8A, data.len() as u8];
                pong.extend_from_slice(&data);
                let _ = stream.write_all(&pong);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obs_authentication_is_stable_and_password_sensitive() {
        let first = obs_authentication("secret", "salt", "challenge");
        let second = obs_authentication("secret", "salt", "challenge");
        assert_eq!(first, second);
        assert_ne!(first, obs_authentication("other", "salt", "challenge"));
        assert!(!first.is_empty());
    }

    #[test]
    fn obs_url_accepts_a_host_and_port() {
        assert_eq!(parse_obs_url("ws://127.0.0.1:4455").unwrap(), ("127.0.0.1".into(), 4455));
        assert_eq!(parse_obs_url("localhost").unwrap(), ("localhost".into(), 4455));
    }
}
