//! Reads `.sdPlugin` folders: the Stream Deck / Stream Dock plugin format.

use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Runtime {
    /// A JavaScript file run with Node.js.
    Node,
    /// An HTML page run in a hidden browser window.
    Html,
    /// A Windows program.
    Exe,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ActionState {
    pub image: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PluginAction {
    pub uuid: String,
    pub name: String,
    pub tooltip: String,
    pub icon: String,
    pub states: Vec<ActionState>,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub uuid: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub icon: String,
    pub dir: PathBuf,
    /// Where it was found: "Stream Dock", "Stream Deck" or "DeckForge".
    pub source: String,
    pub code: Option<PathBuf>,
    pub runtime: Runtime,
    pub actions: Vec<PluginAction>,
}

fn text(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

/// The program to run, only when it is inside the plugin's own folder.
fn code_path(dir: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.trim().replace('/', "\\");
    if rel.is_empty() || rel.contains("..") || Path::new(&rel).is_absolute() {
        return None;
    }
    Some(dir.join(rel))
}

pub fn runtime_of(code: &Path) -> Runtime {
    match code.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("js" | "mjs" | "cjs") => Runtime::Node,
        Some("html" | "htm") => Runtime::Html,
        Some("exe") => Runtime::Exe,
        _ => Runtime::Unknown,
    }
}

pub fn parse(dir: &Path, raw: &str, source: &str) -> Option<Manifest> {
    let value: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok()?;
    let folder = dir.file_name()?.to_string_lossy().to_string();
    let mut uuid = text(&value, "UUID");
    if uuid.is_empty() {
        uuid = folder.trim_end_matches(".sdPlugin").to_string();
    }
    let rel = [text(&value, "CodePathWin"), text(&value, "CodePath")].into_iter().find(|p| !p.is_empty());
    // Stream Deck lets Windows plugins name their program without ".exe".
    let code = rel.and_then(|rel| code_path(dir, &rel)).map(|code| {
        let exe = code.with_extension("exe");
        if code.extension().is_none() && exe.is_file() { exe } else { code }
    });
    let runtime = code.as_deref().map(runtime_of).unwrap_or(Runtime::Unknown);
    let actions = value
        .get("Actions")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|act| {
                    let uuid = text(act, "UUID");
                    if uuid.is_empty() {
                        return None;
                    }
                    let states = act
                        .get("States")
                        .and_then(Value::as_array)
                        .map(|states| {
                            states
                                .iter()
                                .map(|s| ActionState {
                                    image: text(s, "Image"),
                                    title: s.get("Title").and_then(Value::as_str).map(str::to_string),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    Some(PluginAction {
                        name: { let n = text(act, "Name"); if n.is_empty() { uuid.clone() } else { n } },
                        uuid,
                        tooltip: text(act, "Tooltip"),
                        icon: text(act, "Icon"),
                        states,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Manifest {
        name: { let n = text(&value, "Name"); if n.is_empty() { uuid.clone() } else { n } },
        uuid,
        author: text(&value, "Author"),
        version: text(&value, "Version"),
        description: text(&value, "Description"),
        icon: text(&value, "Icon"),
        dir: dir.to_path_buf(),
        source: source.to_string(),
        code,
        runtime,
        actions,
    })
}

/// Folders plugins are installed in, with a name for each.
pub fn plugin_roots() -> Vec<(PathBuf, &'static str)> {
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default();
    vec![
        (appdata.join("DeckForge").join("plugins"), "DeckForge"),
        (appdata.join("HotSpot").join("StreamDock").join("plugins"), "Stream Dock"),
        (appdata.join("Elgato").join("StreamDeck").join("Plugins"), "Stream Deck"),
    ]
}

pub fn scan_roots(roots: &[(PathBuf, &str)]) -> Vec<Manifest> {
    let mut found: Vec<Manifest> = Vec::new();
    for (root, source) in roots {
        let Ok(entries) = std::fs::read_dir(root) else { continue };
        let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        dirs.sort();
        for dir in dirs {
            let Ok(raw) = std::fs::read_to_string(dir.join("manifest.json")) else { continue };
            if let Some(manifest) = parse(&dir, &raw, source) {
                if !found.iter().any(|m| m.uuid.eq_ignore_ascii_case(&manifest.uuid)) {
                    found.push(manifest);
                }
            }
        }
    }
    found
}

/// Reads an image named in a manifest. Manifests leave out the extension and offer `@2x`.
pub fn read_image(dir: &Path, rel: &str) -> Option<Vec<u8>> {
    let rel = rel.trim().replace('/', "\\");
    if rel.is_empty() || rel.contains("..") {
        return None;
    }
    let base = dir.join(&rel);
    let has_ext = Path::new(&rel).extension().is_some();
    let mut tries = Vec::new();
    if has_ext {
        tries.push(base.clone());
    }
    for ext in ["png", "jpg", "gif"] {
        tries.push(PathBuf::from(format!("{}@2x.{ext}", base.display())));
        tries.push(PathBuf::from(format!("{}.{ext}", base.display())));
    }
    tries.into_iter().find_map(|p| std::fs::read(p).ok())
}

/// Finds `node.exe`: on PATH first, then the usual install folder.
pub fn find_node() -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(pf) = std::env::var_os("ProgramFiles") {
        dirs.push(PathBuf::from(pf).join("nodejs"));
    }
    dirs.into_iter().map(|d| d.join("node.exe")).find(|p| p.is_file())
}
