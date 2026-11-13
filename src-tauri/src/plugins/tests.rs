use super::manifest::*;
use super::*;
use std::sync::mpsc;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("deckforge-plugins-{name}-{}-{}", std::process::id(), now_ms()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const MANIFEST: &str = r#"{
  "Name": "Counter", "Author": "Me", "Version": "1.2.0", "CodePath": "bin/plugin.js",
  "Actions": [
    { "UUID": "com.me.counter.add", "Name": "Add one", "Icon": "imgs/add",
      "States": [ { "Image": "imgs/off" }, { "Image": "imgs/on", "Title": "ON" } ] },
    { "Name": "No uuid, skipped" }
  ]
}"#;

fn sample_plugin() -> (PathBuf, Manifest) {
    let root = temp_dir("scan");
    let dir = root.join("com.me.counter.sdPlugin");
    std::fs::create_dir_all(dir.join("imgs")).unwrap();
    std::fs::write(dir.join("manifest.json"), MANIFEST).unwrap();
    std::fs::write(dir.join("imgs/off@2x.png"), [1u8, 2]).unwrap();
    std::fs::write(dir.join("imgs/on.png"), [3u8]).unwrap();
    let found = scan_roots(&[(root.clone(), "Test")]);
    assert_eq!(found.len(), 1);
    (root, found.into_iter().next().unwrap())
}

#[test]
fn manifest_falls_back_to_folder_name_and_detects_the_runtime() {
    let (root, m) = sample_plugin();
    assert_eq!(m.uuid, "com.me.counter");
    assert_eq!(m.runtime, Runtime::Node);
    assert_eq!(m.actions.len(), 1);
    assert_eq!(m.actions[0].states[1].title.as_deref(), Some("ON"));
    assert_eq!(read_image(&m.dir, "imgs/off"), Some(vec![1, 2]));
    assert_eq!(read_image(&m.dir, "imgs/on"), Some(vec![3]));
    assert_eq!(read_image(&m.dir, "../secret"), None);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn code_outside_the_plugin_folder_is_refused() {
    let dir = temp_dir("escape").join("x.sdPlugin");
    std::fs::create_dir_all(&dir).unwrap();
    let m = parse(&dir, r#"{"UUID":"x","CodePath":"..\\..\\evil.exe"}"#, "Test").unwrap();
    assert!(m.code.is_none());
    assert_eq!(m.runtime, Runtime::Unknown);
    assert_eq!(runtime_of(std::path::Path::new("a/index.html")), Runtime::Html);
    assert_eq!(runtime_of(std::path::Path::new("a/run.exe")), Runtime::Exe);
}

#[test]
fn data_url_images_decode_and_svg_is_skipped() {
    assert_eq!(decode_image("data:image/png;base64,AQID"), Some(vec![1, 2, 3]));
    assert_eq!(decode_image("data:image/svg+xml;base64,AQID"), None);
    assert_eq!(decode_image("data:image/svg+xml;charset=utf8,<svg/>"), None);
    assert_eq!(decode_image("imgs/on"), None);
}

#[test]
fn keys_on_the_page_reach_the_plugin_and_its_replies_change_the_face() {
    let (root, m) = sample_plugin();
    let mut h = Host::new(0, vec![m.clone()], Store::default(), None);
    // Pretend the plugin is running and connected.
    h.running.insert(m.uuid.clone(), Running::Window("test".into()));
    let (tx, rx) = mpsc::channel();
    assert!(h.register(&m.uuid, 1, tx));
    assert!(rx.try_recv().unwrap().contains("deviceDidConnect"));

    let mut want = BTreeMap::new();
    want.insert("p.page.6".to_string(), ("com.me.counter.add".to_string(), 6u8));
    want.insert("p.page.1".to_string(), ("com.other.unknown".to_string(), 1u8));
    h.sync_visible(want.clone());
    let appear: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
    assert_eq!(appear["event"], "willAppear");
    assert_eq!(appear["payload"]["coordinates"]["column"], 1);
    assert_eq!(appear["payload"]["coordinates"]["row"], 1);
    assert!(rx.try_recv().is_err(), "unknown actions are not sent anywhere");
    assert_eq!(h.faces["p.page.6"].image, vec![1, 2]);

    // Same page again: nothing new is sent.
    h.sync_visible(want);
    assert!(rx.try_recv().is_err());

    let ctx = "p.page.6";
    h.handle(&m.uuid, &json!({"event":"setTitle","context":ctx,"payload":{"title":"7"}}));
    h.handle(&m.uuid, &json!({"event":"setState","context":ctx,"payload":{"state":1}}));
    assert_eq!(h.faces[ctx].image, vec![3]);
    assert_eq!(h.faces[ctx].title.as_deref(), Some("ON"));
    h.handle(&m.uuid, &json!({"event":"setImage","context":ctx,"payload":{"image":"data:image/png;base64,CQk="}}));
    assert_eq!(h.faces[ctx].image, vec![9, 9]);
    h.handle(&m.uuid, &json!({"event":"setSettings","context":ctx,"payload":{"count":3}}));
    h.handle(&m.uuid, &json!({"event":"getSettings","context":ctx}));
    let reply: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
    assert_eq!(reply["event"], "didReceiveSettings");
    assert_eq!(reply["payload"]["settings"]["count"], 3);

    // Another plugin can't touch this key.
    h.handle("com.someone.else", &json!({"event":"setTitle","context":ctx,"payload":{"title":"x"}}));
    assert_eq!(h.faces[ctx].title.as_deref(), Some("ON"));
    assert_eq!(h.handle(&m.uuid, &json!({"event":"openUrl","payload":{"url":"file:///c:/x"}})), None);
    assert_eq!(h.handle(&m.uuid, &json!({"event":"openUrl","payload":{"url":"https://a.b"}})).as_deref(), Some("https://a.b"));

    // Page changes: the key goes away.
    h.sync_visible(BTreeMap::new());
    let gone: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
    assert_eq!(gone["event"], "willDisappear");
    assert!(h.faces.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_plugin_that_was_not_started_cannot_register() {
    let (root, m) = sample_plugin();
    let mut h = Host::new(0, vec![m.clone()], Store::default(), None);
    let (tx, _rx) = mpsc::channel();
    assert!(!h.register(&m.uuid, 1, tx));
    std::fs::remove_dir_all(root).unwrap();
}

/// Starts a real Node.js plugin from the Stream Dock folder and waits for it to connect.
/// Run with `cargo test --lib real_node_plugin -- --ignored --nocapture`.
#[test]
#[ignore = "needs the Stream Dock speed test plugin and Node.js on this PC"]
fn real_node_plugin_connects_and_takes_a_key_press() {
    let found = scan_roots(&plugin_roots());
    let m = found.into_iter().find(|m| m.uuid.ends_with("speedTest")).expect("speedTest plugin");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let action = m.actions[0].uuid.clone();
    let uuid = m.uuid.clone();
    let store = Store { enabled: vec![uuid.clone()], ..Store::default() };
    let _ = HOST.set(Mutex::new(Host::new(port, vec![m], store, None)));
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || serve(stream));
        }
    });
    let mut want = BTreeMap::new();
    want.insert("t.p.0".to_string(), (action, 0u8));
    host().unwrap().sync_visible(want);
    host().unwrap().start_plugin(&uuid);
    let mut connected = false;
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(100));
        let h = host().unwrap();
        if let Some(p) = h.problems.get(&uuid) { panic!("{p}"); }
        if h.conns.contains_key(&uuid) { connected = true; break; }
    }
    assert!(connected, "plugin never registered");
    let pressed = key_press(Some(0));
    println!("press: {pressed:?}");
    assert!(pressed.is_ok());
    std::thread::sleep(Duration::from_millis(1500));
    let face = host().unwrap().faces["t.p.0"].clone();
    println!("face after press: title={:?} image bytes={}", face.title, face.image.len());
    host().unwrap().stop_plugin(&uuid);
}
