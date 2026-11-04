# DeckForge (`OpenStreamDeck`)

Open-source universal Stream Deck & Stream Dock desktop controller built with **Tauri v2**, **Rust**, **React 19**, **TypeScript**, **Tailwind CSS v4**, and **HIDAPI (`windows-native`)**.

## Highlights
- **Verified Ajazz AKP05E Pro (`VID_0300&PID_3013`, V4 1024-Byte Payload Protocol)**:
  - Hardware-verified `1025`-byte (`0x401`) HID output report framing (`CRT\0\0HAN`, `CRT\0\0LIG`, `CRT\0\0BAT`, `CRT\0\0DRA`, `CRT\0\0STP`, `CRT\0\0CLE`) and `ACK\0\0OK` input listener (`MI_00`).
  - Anti-Freeze **SHA-256 Frame Diffing Cache** (`AntiFreezeFrameCache`) that skips redundant USB JPEG transfers when key visuals haven't changed.
  - Built-in procedural keycap & touch-strip JPEG renderer with crisp vector icons and 5×7 pixel-font labels.
- **Live Win32 Action & Macro Engine**:
  - Real `SendInput` keyboard shortcuts, media keys, system commands, shell launcher, URL opener, and HTTP webhook triggers.
  - Interactive **Custom Actions Builder** with live keybind recording (`● Record Keybind`).
  - Legacy `.sdProfile` importer that scans and converts existing `Stream Dock AJAZZ` profiles.

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
