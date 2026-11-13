//! Runs Stream Deck-style `.sdPlugin` plugins.
//!
//! A plugin is its own program (Node.js script, HTML page or .exe). DeckForge starts it with
//! `-port -pluginUUID -registerEvent -info`, the plugin connects back over a local WebSocket,
//! and the two sides trade JSON events: key presses go out, titles and images come back.
//! Plugins are off until the user turns each one on, because they run with the user's rights.

mod manifest;
#[cfg(test)]
mod tests;

use crate::drivers::hal::now_ms;
use crate::state::{PluginKeyFace, SharedState};
use base64::{engine::general_purpose::STANDARD, Engine as _};
pub use manifest::Runtime;
use manifest::{find_node, plugin_roots, read_image, scan_roots, Manifest};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;

/// Key faces changed; the desktop worker should redraw soon.
static FACE_DIRTY: AtomicBool = AtomicBool::new(false);
static NEXT_CONN: AtomicU64 = AtomicU64::new(1);
static HOST: OnceLock<Mutex<Host>> = OnceLock::new();
/// Kept apart from `Host` so the unit tests never link the window code.
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

pub const ACTION_PREFIX: &str = "plugin:";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Store {
    enabled: Vec<String>,
    /// Per key, by context id.
    settings: BTreeMap<String, SavedSettings>,
    /// Per plugin, by plugin UUID.
    global: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedSettings {
    action: String,
    settings: Value,
}

enum Running {
    Child(std::process::Child),
    Window(String),
}

#[derive(Debug, Clone, PartialEq)]
struct Visible {
    action: String,
    plugin: String,
    index: u8,
}

#[derive(Debug, Clone)]
struct DeviceInfo {
    id: String,
    name: String,
    cols: u8,
    rows: u8,
}

struct Host {
    port: u16,
    manifests: Vec<Manifest>,
    store: Store,
    store_path: Option<PathBuf>,
    conns: HashMap<String, (u64, Sender<String>)>,
    running: HashMap<String, Running>,
    problems: HashMap<String, String>,
    visible: BTreeMap<String, Visible>,
    faces: HashMap<String, PluginKeyFace>,
    device: DeviceInfo,
    #[cfg_attr(test, allow(dead_code))]
    next_window: u64,
    started: bool,
}

impl Host {
    fn new(port: u16, manifests: Vec<Manifest>, store: Store, store_path: Option<PathBuf>) -> Self {
        Host {
            port,
            manifests,
            store,
            store_path,
            conns: HashMap::new(),
            running: HashMap::new(),
            problems: HashMap::new(),
            visible: BTreeMap::new(),
            faces: HashMap::new(),
            device: DeviceInfo { id: "deckforge-deck".into(), name: "DeckForge deck".into(), cols: 5, rows: 2 },
            next_window: 1,
            started: false,
        }
    }

    fn manifest(&self, uuid: &str) -> Option<&Manifest> {
        self.manifests.iter().find(|m| m.uuid == uuid)
    }

    fn owner_of(&self, action: &str) -> Option<&Manifest> {
        self.manifests.iter().find(|m| m.actions.iter().any(|a| a.uuid == action))
    }

    fn save(&self) {
        let Some(path) = &self.store_path else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(raw) = serde_json::to_string_pretty(&self.store) {
            let _ = crate::db::write_atomic(path, &raw);
        }
    }

    fn send(&self, plugin: &str, message: String) {
        if let Some((_, tx)) = self.conns.get(plugin) {
            let _ = tx.send(message);
        }
    }

    fn settings_for(&self, ctx: &str, action: &str) -> Value {
        self.store
            .settings
            .get(ctx)
            .filter(|s| s.action == action)
            .map(|s| s.settings.clone())
            .unwrap_or_else(|| json!({}))
    }

    fn key_event(&self, event: &str, ctx: &str, v: &Visible) -> String {
        let cols = self.device.cols.max(1);
        let state = self.faces.get(ctx).map(|f| f.state).unwrap_or(0);
        json!({
            "event": event,
            "action": v.action,
            "context": ctx,
            "device": self.device.id,
            "payload": {
                "settings": self.settings_for(ctx, &v.action),
                "coordinates": { "column": v.index % cols, "row": v.index / cols },
                "state": state,
                "isInMultiAction": false,
                "controller": "Keypad",
            }
        })
        .to_string()
    }

    /// The image of an action's state, from the plugin folder.
    fn state_face(&self, action: &str, state: u8) -> (Vec<u8>, Option<String>) {
        let Some(m) = self.owner_of(action) else { return (Vec::new(), None) };
        let Some(act) = m.actions.iter().find(|a| a.uuid == action) else { return (Vec::new(), None) };
        let st = act.states.get(state as usize).or_else(|| act.states.first());
        let image = st
            .and_then(|s| read_image(&m.dir, &s.image))
            .or_else(|| read_image(&m.dir, &act.icon))
            .unwrap_or_default();
        (image, st.and_then(|s| s.title.clone()).filter(|t| !t.is_empty()))
    }

    /// Brings the keys plugins see in line with the page on the deck.
    fn sync_visible(&mut self, want: BTreeMap<String, (String, u8)>) {
        let mut next = BTreeMap::new();
        for (ctx, (action, index)) in want {
            if let Some(m) = self.owner_of(&action) {
                next.insert(ctx, Visible { plugin: m.uuid.clone(), action, index });
            }
        }
        let gone: Vec<(String, Visible)> = self
            .visible
            .iter()
            .filter(|(ctx, v)| next.get(*ctx) != Some(*v))
            .map(|(c, v)| (c.clone(), v.clone()))
            .collect();
        let added: Vec<(String, Visible)> = next
            .iter()
            .filter(|(ctx, v)| self.visible.get(*ctx) != Some(*v))
            .map(|(c, v)| (c.clone(), v.clone()))
            .collect();
        if gone.is_empty() && added.is_empty() {
            return;
        }
        for (ctx, v) in &gone {
            self.send(&v.plugin, self.key_event("willDisappear", ctx, v));
            self.faces.remove(ctx);
        }
        for (ctx, v) in &added {
            let (image, title) = self.state_face(&v.action, 0);
            self.faces.insert(ctx.clone(), PluginKeyFace { title, image, state: 0 });
        }
        self.visible = next;
        for (ctx, v) in &added {
            self.send(&v.plugin, self.key_event("willAppear", ctx, v));
        }
        FACE_DIRTY.store(true, Ordering::SeqCst);
    }

    fn info_json(&self, m: &Manifest) -> String {
        json!({
            "application": { "font": "", "language": "en", "platform": "windows", "platformVersion": "10.0", "version": "6.9.0" },
            "plugin": { "uuid": m.uuid, "version": m.version },
            "devicePixelRatio": 2,
            "colors": {
                "buttonPressedBackgroundColor": "#303030FF", "buttonPressedBorderColor": "#646464FF",
                "buttonPressedTextColor": "#969696FF", "disabledColor": "#F7821B59",
                "highlightColor": "#F7821BFF", "mouseDownColor": "#CF6304FF"
            },
            "devices": [{
                "id": self.device.id, "name": self.device.name, "type": 0,
                "size": { "columns": self.device.cols, "rows": self.device.rows }
            }]
        })
        .to_string()
    }

    fn start_plugin(&mut self, uuid: &str) {
        if self.running.contains_key(uuid) || self.port == 0 {
            return;
        }
        let Some(m) = self.manifest(uuid).cloned() else { return };
        self.problems.remove(uuid);
        let Some(code) = m.code.clone().filter(|c| c.is_file()) else {
            self.problems.insert(uuid.into(), "The plugin's program file is missing.".into());
            return;
        };
        let info = self.info_json(&m);
        let args = ["-port".to_string(), self.port.to_string(), "-pluginUUID".into(), m.uuid.clone(), "-registerEvent".into(), "registerPlugin".into(), "-info".into(), info.clone()];
        let result = match m.runtime {
            Runtime::Node => match find_node() {
                Some(node) => spawn_hidden(&node, Some(&code), &args, &m.dir).map(Running::Child),
                None => Err("Needs Node.js. Install it from nodejs.org, then turn this plugin off and on.".into()),
            },
            Runtime::Exe => spawn_hidden(&code, None, &args, &m.dir).map(Running::Child),
            Runtime::Html => self.open_html(&m, &code, &info).map(Running::Window),
            Runtime::Unknown => Err("DeckForge can't run this kind of plugin program.".into()),
        };
        match result {
            Ok(run) => {
                self.running.insert(uuid.into(), run);
            }
            Err(err) => {
                self.problems.insert(uuid.into(), err);
            }
        }
    }

    #[cfg(test)]
    fn open_html(&mut self, _m: &Manifest, _code: &std::path::Path, _info: &str) -> Result<String, String> {
        // Linking the window builder into the test binary stops it from starting on Windows.
        Err("HTML plugins are not started in tests.".into())
    }

    #[cfg(not(test))]
    fn open_html(&mut self, m: &Manifest, code: &std::path::Path, info: &str) -> Result<String, String> {
        let app = APP.get().cloned().ok_or("The app is still starting.")?;
        let url = tauri::Url::from_file_path(code).map_err(|_| "Bad plugin path".to_string())?;
        let label = format!("plugin-{}", self.next_window);
        self.next_window += 1;
        let script = format!(
            "(function(){{var f=window.connectElgatoStreamDeckSocket||window.connectSocket||window.connectStreamDockSocket;if(f)f({},{},'registerPlugin',{});}})();",
            self.port,
            serde_json::to_string(&m.uuid).unwrap_or_default(),
            serde_json::to_string(info).unwrap_or_default(),
        );
        tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(url))
            .visible(false)
            .skip_taskbar(true)
            .title(format!("{} (plugin)", m.name))
            .on_page_load(move |window, payload| {
                if payload.event() == tauri::webview::PageLoadEvent::Finished {
                    let _ = window.eval(&script);
                }
            })
            .build()
            .map_err(|e| e.to_string())?;
        Ok(label)
    }

    fn stop_plugin(&mut self, uuid: &str) {
        self.conns.remove(uuid);
        match self.running.remove(uuid) {
            Some(Running::Child(mut child)) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            Some(Running::Window(label)) => {
                #[cfg(test)]
                let _ = label;
                #[cfg(not(test))]
                if let Some(app) = APP.get() {
                    use tauri::Manager;
                    if let Some(w) = app.get_webview_window(&label) {
                        let _ = w.destroy();
                    }
                }
            }
            None => {}
        }
    }

    /// Notes plugins whose program has quit.
    fn reap(&mut self) {
        let mut ended = Vec::new();
        for (uuid, run) in self.running.iter_mut() {
            if let Running::Child(child) = run {
                if let Ok(Some(status)) = child.try_wait() {
                    ended.push((uuid.clone(), status.code()));
                }
            }
        }
        for (uuid, code) in ended {
            self.running.remove(&uuid);
            self.conns.remove(&uuid);
            let text = match code {
                Some(c) => format!("The plugin stopped (exit code {c}). Turn it off and on to try again."),
                None => "The plugin stopped. Turn it off and on to try again.".into(),
            };
            self.problems.insert(uuid, text);
        }
    }

    /// Handles one message from a registered plugin. Returns a URL to open, if any.
    fn handle(&mut self, plugin: &str, msg: &Value) -> Option<String> {
        let event = msg.get("event").and_then(Value::as_str).unwrap_or("");
        let ctx = msg.get("context").and_then(Value::as_str).unwrap_or("").to_string();
        let payload = msg.get("payload").cloned().unwrap_or(Value::Null);
        let owned = self.visible.get(&ctx).filter(|v| v.plugin == plugin).cloned();
        match event {
            "setTitle" => {
                if owned.is_some() {
                    let title = payload.get("title").and_then(Value::as_str).map(str::to_string);
                    if let Some(face) = self.faces.get_mut(&ctx) {
                        face.title = title;
                        FACE_DIRTY.store(true, Ordering::SeqCst);
                    }
                }
            }
            "setImage" => {
                if let Some(v) = owned {
                    let raw = payload.get("image").and_then(Value::as_str).unwrap_or("");
                    let dir = self.manifest(plugin).map(|m| m.dir.clone());
                    let image = if raw.is_empty() {
                        let state = self.faces.get(&ctx).map(|f| f.state).unwrap_or(0);
                        Some(self.state_face(&v.action, state).0)
                    } else {
                        decode_image(raw).or_else(|| dir.and_then(|d| read_image(&d, raw)))
                    };
                    if let (Some(image), Some(face)) = (image, self.faces.get_mut(&ctx)) {
                        face.image = image;
                        FACE_DIRTY.store(true, Ordering::SeqCst);
                    }
                }
            }
            "setState" => {
                if let Some(v) = owned {
                    let state = payload.get("state").and_then(Value::as_u64).unwrap_or(0).min(15) as u8;
                    let (image, title) = self.state_face(&v.action, state);
                    if let Some(face) = self.faces.get_mut(&ctx) {
                        face.state = state;
                        face.image = image;
                        if title.is_some() {
                            face.title = title;
                        }
                        FACE_DIRTY.store(true, Ordering::SeqCst);
                    }
                }
            }
            "setSettings" => {
                let action = owned.map(|v| v.action).or_else(|| self.store.settings.get(&ctx).map(|s| s.action.clone()));
                if let Some(action) = action {
                    self.store.settings.insert(ctx, SavedSettings { action, settings: payload });
                    self.save();
                }
            }
            "getSettings" => {
                if let Some(v) = owned {
                    let reply = self.key_event("didReceiveSettings", &ctx, &v);
                    self.send(plugin, reply);
                }
            }
            "setGlobalSettings" => {
                self.store.global.insert(plugin.to_string(), payload);
                self.save();
            }
            "getGlobalSettings" => {
                let settings = self.store.global.get(plugin).cloned().unwrap_or_else(|| json!({}));
                let reply = json!({ "event": "didReceiveGlobalSettings", "payload": { "settings": settings } }).to_string();
                self.send(plugin, reply);
            }
            "openUrl" => {
                let url = payload.get("url").and_then(Value::as_str).unwrap_or("").trim();
                if url.starts_with("https://") || url.starts_with("http://") {
                    return Some(url.to_string());
                }
            }
            _ => {}
        }
        None
    }

    /// A plugin connected and said who it is.
    fn register(&mut self, plugin: &str, conn: u64, tx: Sender<String>) -> bool {
        if !self.running.contains_key(plugin) {
            return false;
        }
        self.conns.insert(plugin.to_string(), (conn, tx));
        self.problems.remove(plugin);
        let connect = json!({
            "event": "deviceDidConnect",
            "device": self.device.id,
            "deviceInfo": { "name": self.device.name, "type": 0, "size": { "columns": self.device.cols, "rows": self.device.rows } }
        })
        .to_string();
        self.send(plugin, connect);
        let mine: Vec<(String, Visible)> = self.visible.iter().filter(|(_, v)| v.plugin == plugin).map(|(c, v)| (c.clone(), v.clone())).collect();
        for (ctx, v) in mine {
            self.send(plugin, self.key_event("willAppear", &ctx, &v));
        }
        true
    }

    fn view(&self) -> Vec<PluginView> {
        self.manifests
            .iter()
            .map(|m| PluginView {
                uuid: m.uuid.clone(),
                name: m.name.clone(),
                author: m.author.clone(),
                version: m.version.clone(),
                description: m.description.clone(),
                source: m.source.clone(),
                dir: m.dir.to_string_lossy().to_string(),
                runtime: m.runtime,
                enabled: self.store.enabled.contains(&m.uuid),
                running: self.conns.contains_key(&m.uuid),
                problem: self.problems.get(&m.uuid).cloned(),
                icon: read_image(&m.dir, &m.icon).map(|bytes| data_url(&bytes)),
                actions: m
                    .actions
                    .iter()
                    .map(|a| ActionView { uuid: a.uuid.clone(), name: a.name.clone(), tooltip: a.tooltip.clone() })
                    .collect(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionView {
    pub uuid: String,
    pub name: String,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginView {
    pub uuid: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub source: String,
    pub dir: String,
    pub runtime: Runtime,
    pub enabled: bool,
    /// Connected to DeckForge right now.
    pub running: bool,
    pub problem: Option<String>,
    pub icon: Option<String>,
    pub actions: Vec<ActionView>,
}

fn data_url(bytes: &[u8]) -> String {
    let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if bytes.starts_with(b"GIF") {
        "image/gif"
    } else {
        "image/jpeg"
    };
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

/// Decodes a `data:image/...;base64,` image. SVG is not supported and gives `None`.
pub(crate) fn decode_image(raw: &str) -> Option<Vec<u8>> {
    let rest = raw.trim().strip_prefix("data:")?;
    let (head, body) = rest.split_once(',')?;
    if head.contains("svg") || !head.ends_with(";base64") {
        return None;
    }
    STANDARD.decode(body.trim()).ok().filter(|b| !b.is_empty())
}

fn host() -> Option<MutexGuard<'static, Host>> {
    HOST.get().map(|m| m.lock().unwrap_or_else(|e| e.into_inner()))
}

fn store_path() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("DeckForge")
        .join("plugins.json")
}

#[cfg(all(target_os = "windows", not(test)))]
fn kill_on_exit_job() -> Option<isize> {
    use windows::Win32::System::JobObjects::*;
    static JOB: OnceLock<Option<isize>> = OnceLock::new();
    *JOB.get_or_init(|| unsafe {
        let job = CreateJobObjectW(None, None).ok()?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .ok()?;
        Some(job.0 as isize)
    })
}

/// Starts a plugin program with no console window. It is tied to DeckForge, so it ends
/// when DeckForge does.
fn spawn_hidden(program: &std::path::Path, script: Option<&std::path::Path>, args: &[String], dir: &std::path::Path) -> Result<std::process::Child, String> {
    let mut cmd = std::process::Command::new(program);
    if let Some(script) = script {
        cmd.arg(script);
    }
    cmd.args(args).current_dir(dir).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let child = cmd.spawn().map_err(|e| format!("Could not start the plugin: {e}"))?;
    #[cfg(all(target_os = "windows", not(test)))]
    if let Some(job) = kill_on_exit_job() {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::HANDLE;
        unsafe {
            let _ = windows::Win32::System::JobObjects::AssignProcessToJobObject(HANDLE(job as *mut _), HANDLE(child.as_raw_handle()));
        }
    }
    Ok(child)
}

fn serve(stream: TcpStream) {
    let _ = stream.set_nodelay(true);
    let Ok(mut ws) = tungstenite::accept(stream) else { return };
    let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(30)));
    let conn = NEXT_CONN.fetch_add(1, Ordering::SeqCst);
    let (tx, rx) = mpsc::channel::<String>();
    let mut tx = Some(tx);
    let mut plugin: Option<String> = None;
    'outer: loop {
        match ws.read() {
            Ok(tungstenite::Message::Text(text)) => {
                let Ok(msg) = serde_json::from_str::<Value>(&text) else { continue };
                let event = msg.get("event").and_then(Value::as_str).unwrap_or("");
                match &plugin {
                    None if event == "registerPlugin" => {
                        let uuid = msg.get("uuid").and_then(Value::as_str).unwrap_or("").to_string();
                        let Some(sender) = tx.take() else { break };
                        let ok = host().map(|mut h| h.register(&uuid, conn, sender)).unwrap_or(false);
                        if !ok {
                            break;
                        }
                        plugin = Some(uuid);
                    }
                    Some(uuid) => {
                        let open = host().and_then(|mut h| h.handle(uuid, &msg));
                        if let Some(url) = open {
                            let _ = crate::actions::launch_app_or_url(&url);
                        }
                    }
                    None => {}
                }
            }
            Ok(tungstenite::Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(_) => break,
        }
        if plugin.is_some() {
            loop {
                match rx.try_recv() {
                    Ok(out) => {
                        if ws.send(tungstenite::Message::Text(out)).is_err() {
                            break 'outer;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break 'outer,
                }
            }
        }
    }
    let _ = ws.close(None);
    if let (Some(uuid), Some(mut h)) = (plugin, host()) {
        if h.conns.get(&uuid).map(|(id, _)| *id) == Some(conn) {
            h.conns.remove(&uuid);
        }
    }
}

/// Starts the WebSocket server and the thread that keeps plugins in step with the deck.
pub fn start(app: tauri::AppHandle, shared: SharedState) {
    let listener = TcpListener::bind("127.0.0.1:0").ok();
    let port = listener.as_ref().and_then(|l| l.local_addr().ok()).map(|a| a.port()).unwrap_or(0);
    let path = store_path();
    let store = std::fs::read_to_string(&path).ok().and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default();
    let initial = Host::new(port, scan_roots(&plugin_roots()), store, Some(path));
    let _ = APP.set(app);
    if HOST.set(Mutex::new(initial)).is_err() {
        return;
    }
    if let Some(listener) = listener {
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || serve(stream));
            }
        });
    }
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(250));
        let (want, device) = {
            let guard = shared.lock().unwrap_or_else(|e| e.into_inner());
            let device = guard
                .devices
                .iter()
                .find(|d| d.connected && d.protocol_family.starts_with("CRT"))
                .map(|d| DeviceInfo { id: d.id.clone(), name: d.model_name.clone(), cols: d.keys.cols.max(1), rows: d.keys.rows.max(1) });
            let mut want = BTreeMap::new();
            if guard.layout_ready {
                if let Some(layout) = &guard.applied_layout {
                    for key in &layout.page.keys {
                        if let Some(action) = key.action_id.strip_prefix(ACTION_PREFIX) {
                            let ctx = format!("{}.{}.{}", layout.profile_id, layout.page.id, key.index);
                            want.insert(ctx, (action.to_string(), key.index));
                        }
                    }
                }
            }
            (want, device)
        };
        if let Some(mut h) = host() {
            if let Some(device) = device {
                h.device = device;
            }
            if !h.started {
                h.started = true;
                for uuid in h.store.enabled.clone() {
                    h.start_plugin(&uuid);
                }
            }
            h.reap();
            h.sync_visible(want);
        }
        if FACE_DIRTY.swap(false, Ordering::SeqCst) {
            shared.lock().unwrap_or_else(|e| e.into_inner()).sample_after_ms = now_ms();
        }
    });
}

pub fn list() -> Vec<PluginView> {
    host().map(|h| h.view()).unwrap_or_default()
}

/// Looks for newly installed plugins. Running ones keep running.
pub fn rescan() -> Vec<PluginView> {
    let Some(mut h) = host() else { return Vec::new() };
    let found = scan_roots(&plugin_roots());
    for m in found {
        if !h.manifests.iter().any(|old| old.uuid == m.uuid) {
            h.manifests.push(m);
        }
    }
    h.view()
}

pub fn set_enabled(uuid: &str, enabled: bool) -> Result<Vec<PluginView>, String> {
    let mut h = host().ok_or("Plugins are still starting.")?;
    if h.manifest(uuid).is_none() {
        return Err("That plugin is not installed.".into());
    }
    h.store.enabled.retain(|u| u != uuid);
    if enabled {
        h.store.enabled.push(uuid.to_string());
        h.start_plugin(uuid);
    } else {
        h.stop_plugin(uuid);
        h.problems.remove(uuid);
    }
    h.save();
    Ok(h.view())
}

/// Sends a key press to the plugin that owns the key on the deck.
pub fn key_press(index: Option<u8>) -> Result<String, String> {
    let index = index.ok_or("Press the key on the deck to try a plugin action.")?;
    let h = host().ok_or("Plugins are still starting.")?;
    let (ctx, v) = h
        .visible
        .iter()
        .find(|(_, v)| v.index == index)
        .map(|(c, v)| (c.clone(), v.clone()))
        .ok_or("This plugin is not installed on this PC.")?;
    let name = h.manifest(&v.plugin).map(|m| m.name.clone()).unwrap_or_else(|| v.plugin.clone());
    if !h.store.enabled.contains(&v.plugin) {
        return Err(format!("Turn on the {name} plugin on the Plugins page first."));
    }
    if !h.conns.contains_key(&v.plugin) {
        return Err(h.problems.get(&v.plugin).cloned().unwrap_or_else(|| format!("{name} is still starting.")));
    }
    h.send(&v.plugin, h.key_event("keyDown", &ctx, &v));
    h.send(&v.plugin, h.key_event("keyUp", &ctx, &v));
    Ok(format!("Sent to {name}"))
}

/// What plugins want shown on the keys of the page on the deck, by key index.
pub fn current_faces() -> BTreeMap<u8, PluginKeyFace> {
    let Some(h) = host() else { return BTreeMap::new() };
    h.visible
        .iter()
        .filter_map(|(ctx, v)| h.faces.get(ctx).map(|f| (v.index, f.clone())))
        .collect()
}
