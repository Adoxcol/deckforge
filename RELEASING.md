# Release checks

Run these checks before tagging a new DeckForge version. Keep the release tied to a passing commit on `main`.

1. Confirm the working tree has no unintended changes. Update the version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
2. Run `npm run build`, `npm test`, and `cargo test --locked --lib` in `src-tauri` using the toolchain in `rust-toolchain.toml`. Wait for the Windows CI release build to pass.
3. On an AKP05E Pro, check a cold start, key press, knob turn and push, touch strip tap and swipe, page/profile switch, LED setting, full layout push, USB unplug/reconnect, and tray reopen/quit. Check that actions still work after the editor is hidden.
4. Exercise any changed feature on the installed build. For audio or Game Mode changes, use a disposable test setup and restore the original settings afterward.
5. Tag the validated commit. Wait for the Release workflow, then verify that the installer, `.sig` file, and `latest.json` are published. Confirm the manifest version and Windows download URL match the tag before marking the release ready.
6. Write release notes that distinguish automated test results from physical hardware checks. Record any device families that have protocol code but have not been verified on hardware.
