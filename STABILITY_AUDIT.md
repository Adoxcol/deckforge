# DeckForge stability audit

Reviewed 2026-11-12 against the current working copy, including the seven pre-existing modified Rust files. The initial review changed no application code. Implementation progress is recorded below.

## Fix progress

- **Issue 1 implemented (2026-11-12):** editor navigation no longer sends a backend context mutation. Physical bindings and live face updates use a snapshot of the successfully applied page, independent of subsequent saved edits. Full layout changes serialize against animations/live redraws, suppress new binding resolution during transfer, and commit bindings/playback only after transport success. Invalid targets are rejected; failed transfers leave dispatch disabled until a successful retry. The editor no longer starts by claiming a successful sync.
- Added five isolated Rust regression tests covering saved key/encoder/touch edits, edits during transmission, partial failure and retry, invalid/disconnected targets, and live-face updates from the applied snapshot. They use an injected transport and do not access real hardware or user configuration.
- Validation after issue 1: frontend production build passed; Rust library suite **52 passed, 0 failed, 3 ignored**; `git diff --check` passed (line-ending warnings only).
- This also improves full-layout failure handling from issue 3, but does **not** complete the animation/frame-cache accounting or USB scheduler fixes. Physical transfer timing still needs hardware verification.
- **Issue 1 committed and pushed:** `4897233` on `origin/main`, including the pre-existing runtime hardening on which it depends. Generated CodeGraph files were excluded.
- **Issue 2 implemented (2026-11-12):** reconnect and Re-send layout now resend the last successfully applied snapshot without resetting the profile/page or loading unapplied saved edits. A versioned `applied-layout.json` persists successful snapshots for application restart. Failed transfers do not replace it; persistence failures produce a warning without claiming the hardware apply failed. Startup editor selection resolves the restored page by ID, including reordered pages.
- Five additional regression tests cover non-default recovery, restart/file replacement, failed applies preserving the prior snapshot, missing/corrupt/unsupported files, persistence failure reporting, and reordered editor pages. Validation: **57 passed, 0 failed, 3 ignored**; frontend production build and diff checks passed. Physical reconnect testing is still pending. Issues 3–10 remain open.
- **Issue 2 committed and pushed:** `651e539` on `origin/main`.
- **Issue 3 implemented (2026-11-12):** frame planning now stages hashes; full-layout and background-frame batches commit hashes, transmitted counts, image-report byte counts and USB latency only after successful transport completion. Failed batches invalidate the affected device's cache without adding successful-transfer counts. Live-face errors schedule a retry even when OS values do not change; animation errors restore playback position with backoff, including the final frame of non-looping animations. Disconnected devices and short HID writes return errors. The legacy hardware-test sender uses the shared transport; clear-key write errors propagate to the caller.
- Added six regression tests covering pending/failed cache accounting, device isolation, full-layout commit, unchanged live-face retry, final animation-frame retry, disconnected writes and short reports. Validation: **63 passed, 0 failed, 3 ignored**; frontend production build passed. These checks verify transport-result handling, not firmware display acknowledgement. Physical USB fault testing remains pending; queue deadlines/cancellation are issue 4. Issues 4–10 remain open.
- **Issue 3 committed and pushed:** `0e5f8c4` on `origin/main`.
- **Issue 4 implemented (2026-11-12):** a cooperative HID scheduler reads input between packets and polls in at most 10 ms slices during firmware/cooldown delays. Heartbeats run only at safe command/image boundaries, preserving BAT/DRA payload order. Output is limited to eight queued jobs and 2,048 reports per job, with nonblocking overload rejection and 15-second deadlines. Caller timeouts cancel pending work; each packet checks cancellation/deadline, old-connection jobs are rejected, and interrupted transfers close the session for recovery. Recovery requests skip obsolete generations, and suppressed presses no longer enqueue wake writes. Debug-script delays also use the bounded scheduler.
- Added nine deterministic scheduler/queue regression tests covering responsive waits, heartbeat boundaries, cancellation before and during execution, deadline expiry during a write, input failures, size limits, queue overload and connection epochs. Validation: **72 passed, 0 failed, 3 ignored**; frontend build and diff checks passed. No hardware latency claim is made: an OS HID call already in flight cannot be interrupted by this cooperative cancellation, initial connection firmware-settle time remains, and physical USB/load tests are pending. Issues 5–10 remain open.
- **Issue 4 committed and pushed:** `be33efc` on `origin/main`.
- **Issue 5 implemented (2026-11-12):** macro, auto-switch and OBS edits now use serialized save queues with persistent pending/error indicators and a Retry save action. Failed values remain in the editor session; superseded saves cannot mark newer edits saved. Bootstrap failures show an error and Retry loading, with cleanup for subscriptions resolving after unmount/retry. Profile and macro-assignment save failures report errors; profile imports wait for their macros to save before acknowledging success. Backend settings writes serialize under the state lock; OBS uses atomic file replacement and temporary files are flushed before rename.
- Media staging now propagates disk errors before inserting clips into memory. Media bytes are written under content-hash filenames and metadata is atomically switched only after the data write succeeds. Failed metadata publication preserves the prior clip; legacy metadata/bin pairs remain readable. Unreferenced blobs may remain after a failed replacement (safe cleanup is deferred).
- Added three frontend save-queue tests and three Rust media persistence tests, using isolated temporary directories instead of user media storage. Validation: **75 Rust tests passed, 3 ignored; 3 frontend tests passed; production frontend build passed**. CI now runs the frontend tests. Interactive startup/retry UI testing and real disk-full/power-loss testing were not performed; failures were injected through blocked filesystem paths and rejected save promises. Issues 6–10 remain open.

## Assessment

DeckForge has a substantive Windows/AJAZZ AKP05E Pro implementation: HID packet generation, input decoding, reconnect handling, image rendering, native actions, OBS integration, profiles and macros. It is not yet a dependable replacement for the vendor application. The highest risks concern consistency between displayed icons and executed actions, USB scheduling, and persistence failures. More features should wait until these paths are reliable.

The AKP05E Pro is the concrete hardware target in this code. Other catalog entries include simulators and unverified drivers; the product should not imply equivalent universal hardware support. This review did not establish parity with every feature of your installed AJAZZ application.

## Verification

- `npm run build`: passed TypeScript checking and production bundling.
- Rust library tests: **47 passed, 0 failed, 3 ignored**. Ignored tests include physical hardware and vendor-profile integration, plus glyph rendering.
- The default `cargo test --locked` failed because the resolved compiler was Rust 1.89 while locked Tauri dependencies require 1.90. `rustup run 1.90.0` alone still picked up the older compiler. Explicit process-local compiler paths resolved this:

```powershell
cd src-tauri
$env:RUSTC = (rustup which --toolchain 1.90.0 rustc)
$env:RUSTDOC = (rustup which --toolchain 1.90.0 rustdoc)
rustup run 1.90.0 cargo test --locked --lib
```

No physical-device soak test, installer test, or interactive UI test was performed. Passing unit tests does not verify those behaviors. Findings below are established by code-path inspection; the described hardware symptoms remain to be measured on the device.

## Priority findings

### 1. High: editor navigation changes physical bindings before updating icons

Evidence: `src/App.tsx:204`, `src-tauri/src/commands.rs:431`, `src-tauri/src/live.rs:24`.

Changing the editor's selected profile/page calls `set_active_context`. That command changes the context used by `action_for_event` without transmitting a layout. Select another saved page without applying it: the physical display can still show the old page while button presses execute the newly selected page's actions. Saving edits to an active profile can similarly change bindings before a corresponding display update.

Fix: separate editor selection, saved configuration, and applied device state. Publish a new applied revision only through a coordinated display-and-binding transition. Test that editor browsing and saving cannot silently change a physical key's meaning.

### 2. High: reconnect and Re-send layout reset the active profile/page

Evidence: `src-tauri/src/live.rs:397`, `src-tauri/src/drivers/hal.rs:1488`, `src-tauri/src/drivers/hal.rs:120`.

`push_connected_layout` restores `assigned_profile_id` and resets the page index to zero. The AKP05E catalog assignment defaults to `profile-streaming`, and normal profile application does not update that assignment. Re-send increments the same generation used for reconnect, so requesting recovery also resets context. This path does not emit a matching context-change event to the editor.

Fix: preserve the last successfully applied device context across reconnect and explicit resend; persist it across application restarts. Test recovery from a non-default profile and page.

### 3. High: frame cache and sync status commit before successful delivery

Evidence: `src-tauri/src/drivers/frame_cache.rs:38`, `src-tauri/src/commands.rs:141`, `src-tauri/src/live.rs:493`, `src-tauri/src/workers.rs:332`.

`should_transmit` updates hashes and transmitted counters while preparing packets. Applied timestamps and playback state are also updated before the write result. Failed animation/live-face writes are discarded. A failed frame can consequently be treated as already displayed and skipped on an identical retry. Navigation ignores its transmission error as well.

Fix: separate pending frames from committed frames. Commit hashes, timestamps and counters after successful writes; invalidate or retry failed slots. Propagate structured failure status. Add a fake transport that fails partway through a layout and verify a complete retry.

### 4. High: USB output blocks input reads and heartbeat scheduling

Evidence: `src-tauri/src/drivers/hal.rs:1313`, `src-tauri/src/drivers/hal.rs:1383`, `src-tauri/src/drivers/hal.rs:1460`.

The I/O loop drains queued jobs, writes every packet, and sleeps through inter-packet delays before reading input again. Full pushes include substantial settle delays. Continuous queued work can defer reads and the next heartbeat. A 15-second caller timeout does not cancel its queued job, so an old timed-out layout can still execute later.

Fix: bound/coalesce output work, discard superseded layout revisions, implement deadlines/cancellation, and service input/heartbeat between safe protocol boundaries. Measure key latency during full layout transfers and animations.

### 5. High: persistence failures are hidden from the user

Evidence: `src/App.tsx:369`, `src-tauri/src/media.rs:113`, `src-tauri/src/media.rs:146`.

Macro, auto-switch and OBS edits update UI state while swallowing save errors. Media staging ignores `persist_clip` failures and returns success; the media can work until restart and then disappear. Bootstrap also lacks a visible failure/retry path. These make a working session look safely saved when it is not.

Fix: report save failures, retain a visible unsaved state, and only acknowledge durable media staging after persistence succeeds. Exercise denied-write and interrupted-write cases, then restart and verify recovery.

### 6. High: DeckForge force-kills the official application automatically

Evidence: `src-tauri/src/lib.rs:63`, `src-tauri/src/workers.rs:341`, `src-tauri/src/actions/mod.rs:553`.

Startup launches a recurring guard which force-terminates vendor processes and their process trees using `taskkill /F /T`. The opt-out is an undocumented-in-UI marker file. Opening DeckForge while editing in AJAZZ can terminate that session and potentially lose unsaved changes.

Fix: detect competing ownership, explain the conflict, and expose an explicit takeover preference. Make switching back to the vendor application practical. Add single-instance enforcement so two DeckForge processes cannot compete either.

### 7. Medium: legacy plugin discovery is presented as active compatibility

Evidence: `src-tauri/src/db/mod.rs:1041`, `src-tauri/src/db/mod.rs:1069`, `src/screens/PluginsScreen.tsx:96`.

Filesystem discovery marks plugins enabled, hardcodes `plugin_host_active: true`, and advertises a sandboxed worker/WebSocket bridge. The inspected startup contains no corresponding plugin host. Marketplace tabs change their selected styling but do not filter the rendered list. Discovery alone does not make vendor plugin actions executable.

Fix: label packages as discovered/unsupported until runtime compatibility is established. Inventory the plugins actually required by your existing setup before implementing or promising compatibility.

### 8. Medium: macros have no execution limit or cancellation

Evidence: `src-tauri/src/macro_run.rs:86`.

Each invocation creates a new thread. Repeated presses can run overlapping sequences, interleave shortcuts, and accumulate sleeping or network-waiting workers. There is no per-macro concurrency policy or stop control.

Fix: define queue/restart/ignore-while-running behavior, cap concurrency, and add cancellation and execution deadlines. Verify rapid repeated presses and slow webhook steps.

### 9. Medium: startup migrations can rewrite user-authored layouts

Evidence: `src-tauri/src/db/mod.rs:1107`, `src-tauri/src/db/mod.rs:1149`.

Every load runs heuristic migrations. A page is recognized as stock using its first six key titles, so customized actions with unchanged titles can be replaced. Desktop and Music pages are also added whenever those names are absent. The changed profiles are then automatically saved.

Fix: use explicit schema versions and one-time, narrowly scoped migrations with backups. User removal/renaming of stock pages must survive restart.

### 10. Medium: saved-state indicators and background lifecycle are incomplete

Evidence: `src/App.tsx:441`, `src/App.tsx:451`, `src-tauri/src/lib.rs:58`, `src-tauri/Cargo.toml`.

Undo/redo changes profiles but does not mark them locally unsaved. The app has no inspected close-with-unsaved-edits handling, tray lifecycle, autostart integration, or single-instance setup. A deck controller needs an explicit distinction between closing its editor and stopping its background device service.

Fix: derive dirty state from saved revisions; provide save/recovery behavior on close. Add a deliberate background lifecycle and validate login, window close, process exit and restart behavior.

## Stabilization sequence

1. **Correctness first:** separate editor/applied state, preserve reconnect context, make USB failures observable, and commit frame cache only after delivery.
2. **Persistence next:** propagate write failures, serialize saves, version migrations, preserve backups, and include media in a complete backup/restore flow. Current profile export intentionally excludes images/animations.
3. **Runtime reliability:** bounded transport scheduling, cancellation, macro limits, single-instance ownership, and explicit vendor-app takeover.
4. **Product completeness:** background/tray behavior, autostart preference, accurate device/plugin support labels, and workflows for the integrations used in your current setup.
5. **Release discipline:** pin the Rust toolchain, use locked dependencies consistently in CI, add meaningful frontend workflow tests and transport fault tests, and smoke-test the packaged Windows installer. Existing CI builds and runs Rust tests but does not establish installer or hardware reliability.

## Suggested release acceptance criteria

- Reconnect, explicit resend, restart and sleep/resume preserve the expected profile/page and brightness.
- Every displayed key corresponds to its applied action; browsing or saving editor changes never silently changes that mapping.
- An 8-hour mixed idle/animation session and 50 unplug/replug cycles produce no unexplained blank layouts, duplicate actions or lost configuration.
- Physical-input latency is measured under animation and full-layout load; agree on a target such as p95 below 100 ms for ordinary local actions, with intentional macro/network delays reported separately.
- Simulated write failures, disk failures, corrupt stores and rapid repeated edits recover visibly without claiming success or overwriting recoverable data.
- Closing/reopening the editor, launching twice, and switching between DeckForge and AJAZZ have explicit, predictable behavior.
- Your actual AJAZZ profiles, shortcuts, encoder/touch bindings and required integrations pass a written comparison checklist on the physical AKP05E Pro.

These are proposed release gates, not results already achieved by this audit.
