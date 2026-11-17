# DeckForge (`OpenStreamDeck`)

Open-source universal Stream Deck & Stream Dock desktop controller built with **Tauri v2**, **Rust**, **React 19**, **TypeScript**, **Tailwind CSS v4**, and **HIDAPI (`windows-native`)**.

## Highlights
- **Verified Ajazz AKP05E Pro (`VID_0300&PID_3013`, V4 1024-Byte Payload Protocol)**:
  - Hardware-verified `1025`-byte (`0x401`) HID output report framing (`CRT\0\0HAN`, `CRT\0\0LIG`, `CRT\0\0BAT`, `CRT\0\0DRA`, `CRT\0\0STP`, `CRT\0\0CLE`) and `ACK\0\0OK` input listener (`MI_00`).
  - Anti-Freeze **SHA-256 Frame Diffing Cache** (`AntiFreezeFrameCache`) that skips redundant USB JPEG transfers when key visuals haven't changed.
  - Built-in procedural keycap & touch-strip JPEG renderer with crisp vector icons and 5×7 pixel-font labels.
- **Macro execution limits**: one chain runs at a time across hardware and UI triggers; extra starts are rejected. Chains allow up to 256 blocks and 60 seconds total. Block timeouts include retries. Use **Stop running chain** in Macro Studio to cancel waits/webhooks and prevent subsequent blocks. Already-dispatched OS actions cannot be undone.
- **Live Win32 Action & Macro Engine**:
  - Real `SendInput` keyboard shortcuts, media keys, system commands, shell launcher, URL opener, and HTTP webhook triggers.
  - Interactive **Custom Actions Builder** with live keybind recording (`● Record Keybind`).
  - Legacy `.sdProfile` importer that scans and converts existing `Stream Dock AJAZZ` profiles.
  - A **Shortcuts** catalog of ready-made Windows, editing, game-recording and browser hotkeys (each stays editable), plus OBS actions for virtual camera, replay buffer, record pause, studio transition, switch to a named scene and mute a named source.
- **Window and background lifecycle**: closing the editor hides it to the system tray and retains in-memory edits while device control continues. Use the tray to reopen or quit; Quit offers Save and quit, Discard and quit, or Cancel and waits for pending writes. Failed saves keep the app open. Turn off Game Mode before quitting. Settings → General offers opt-in Windows sign-in launch in the tray. Unsaved state is not crash/logout recovery: save before restarting Windows.
- **Saved layouts**: startup preserves existing pages and bindings without guessing which are defaults. Legacy profile arrays remain readable; the first explicit save upgrades only the storage container to schema version 1 and keeps the original bytes in `profiles.pre-v1-N.json` beside `profiles.json`. Corrupt or unsupported newer stores are not overwritten. Older DeckForge builds do not understand the versioned container; keep the backup if downgrading.
- **Sharing**: export a profile (with the macros it uses) or a Game Mode to a JSON file in `Documents\DeckForge Exports`, and import one on another PC. Pictures and animations stay on the PC they came from, and imported Game Modes are limited by the same protected-program list as your own.
- **Game Mode**: build your own modes from what is actually running on your PC (open apps, background processes, services), optionally switch to one of the power plans installed on it, and have everything put back afterwards. Nothing is closed until you pick it, and anti-cheat, drivers, audio, security software and DeckForge itself are never offered.
- **Deck reliability**: heartbeat that keeps the firmware awake, automatic reconnect and re-send after a USB drop, a status badge with a **Re-send layout** button, and explicit **Pause DeckForge / Use DeckForge** controls. If AJAZZ is detected, DeckForge releases the device and waits for you to close AJAZZ and choose **Use DeckForge**. It never terminates the vendor app. A Windows session mutex prevents a second DeckForge instance from starting device workers.

## Per-app audio controls

In the action picker, choose **App Volume Up**, **App Volume Down**, or **Mute App**. Select an app with an existing audio session, or enter its executable name (for example, `spotify.exe`). Use **Refresh apps** after starting audio playback.

For a mixer knob, assign Volume Up to clockwise, Volume Down to counterclockwise, and Mute App to push, using the same executable for all three. Steps are two percentage points, clamped to 0–100%. All accessible sessions for that executable across active playback outputs are targeted. Mixed mute states become all-muted on the next press; an all-muted app becomes unmuted. Assignments survive PID changes. Browser tabs can share the browser executable; individual tabs are not separate targets.

Missing sessions report an error without falling back to master volume. Partial write failures report how many sessions changed. Live app indicators show a percentage, a range when sessions differ, or Muted / MIX (some sessions muted). Keys and touch zones update on the deck; the editor also shows app state for knobs. A single level meter is shown only when the app has one consistent volume/mute state. Background sampling runs when saved or applied profiles use app-audio actions; the open app picker refreshes independently. Missing sessions and sampling failures are shown explicitly. Foreground-application targeting is not yet supported.

## Development

```powershell
npm install
npm run tauri dev
```

### Run Hardware & Protocol Tests
```powershell
cd src-tauri
cargo test --ignore-rust-version -- --nocapture
```

### Media library

Select a key, touch zone, or strip and choose **Browse media library**. The library includes four original CC0 starter animations, search over saved media, favorites, and recent selections. Preview an item, then choose **Use on screen**; adjust fit, speed, and looping in the editor and choose **Apply to Device** when ready.

Upload a local image/GIF or paste a direct HTTPS media URL (up to 8 MB). Link imports run in the desktop app, use bounded downloads, and validate the actual image before saving. Imported media is saved locally for offline reuse. Favorites and recent selections stay in this installation's local browser storage; they are not included in profile backups. The library does not currently search a third-party GIF service.

The procedural starter artwork in `src/lib/starterMedia.ts` is dedicated to the public domain under [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/). This dedication applies to the artwork; the application's code retains its repository license.
