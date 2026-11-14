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

- **Issue 5 committed and pushed:** `ee1c3c8` on `origin/main`.
- **Issue 6 implemented (2026-11-12):** removed automatic vendor process termination and its recurring guard. Detecting AJAZZ/Stream Dock releases the HID handle and latches DeckForge paused; resuming requires closing the vendor app and choosing Use DeckForge. Pause DeckForge releases control for switching back. Queued output and input dispatch are rejected while paused. A Windows session-scoped mutex acquired before state/workers prevents duplicate DeckForge instances.
- Added regression tests for vendor process names, conflict/manual-pause policy, and native duplicate-instance/release behavior. Validation: **79 Rust tests passed, 3 ignored; 3 frontend tests passed; production build passed**. Vendor detection polls once per second; in-flight OS calls cannot be interrupted, pause is session-only, and physical ownership handoff/window-focus testing remains pending. Issues 7–10 remain open.

- **Issue 6 committed and pushed:** `1387820` on `origin/main`.
- **Issue 7 implemented (2026-11-12):** vendor discovery returns disabled packages, no claimed sandbox/bridge permissions, an inactive host, and no listening port. The plugin screen is now an inventory with working Built-in Actions / Discovered Vendor Plugins filters; vendor cards say Unsupported and explain that imported profiles do not enable vendor actions. Browser preview no longer invents discovered vendor counts or an active host.
- An isolated filesystem regression test covers valid/malformed manifests and a missing vendor directory while preserving built-in availability. Validation: **80 Rust tests passed, 3 ignored; 3 frontend tests passed; production build and diff checks passed**. No vendor runtime was implemented or compatibility promised; interactive UI testing remains pending. Issues 8–10 remain open.

- **Issue 7 committed and pushed:** `e258768` on `origin/main`.
- **Issue 8 implemented (2026-11-12):** all production macro entry points reserve one shared execution slot before creating a worker; overlapping starts are rejected without queuing. RAII releases the slot on completion, failure or unwind, and cancellation retains it until execution exits. Chains are limited to 256 steps and a 60-second cooperative deadline; per-block deadlines include retries. Stop running chain cancels delays/webhooks and prevents later blocks even under Continue/RetryOnce. UI execution runs on the blocking worker pool so the command handler can accept Stop; errors and rejection are shown, and pre-run saves use the existing serialized save queue.
- Macro webhooks now own and await their PowerShell child, propagate process failures, and kill/reap it on cancellation or timeout instead of dispatching detached work. Native actions already in flight cannot be undone or forcibly interrupted; deadlines are checked again when they return. Standalone webhook actions outside macros are unchanged.
- Added six regression tests covering simultaneous starts, retained cancellation slots, size limits, cancellation during waits, expired-run suppression, retry deadlines, error policies and real Windows child termination/reaping. Validation: **86 Rust tests passed, 3 ignored; 3 frontend tests passed; frontend production build passed**. Tests used controlled child processes rather than external webhook services. Physical rapid-press and interactive Stop UI testing remain pending. Issues 9–10 remain open.

- **Issue 8 committed and pushed:** `fba4c17` on `origin/main`.
- **Issue 9 implemented (2026-11-12):** removed startup heuristics that replaced pages from key titles, changed navigation targets/touch bindings, and re-added Desktop/Music. Profile loads are read-only. Legacy arrays are accepted as version 0; an explicit save upgrades only the container to schema version 1 after writing and flushing a non-overwriting `profiles.pre-v1-N.json` backup. Future versions, malformed stores and read failures return errors; save refuses to overwrite them. Startup records a diagnostic when it must show defaults. New installations receive defaults without writing them until an explicit save.
- Replaced two tests asserting destructive migrations with six filesystem regression tests covering preserved customized bindings/titles, deleted/renamed pages, repeated loads/saves, byte-exact legacy backups, backup collisions, corrupt/future versions, failed backup/replacement, and first-run behavior. Validation: **90 Rust tests passed, 3 ignored; 3 frontend tests passed; production build and diff checks passed**. Tests use isolated temporary stores; no actual user profiles were rewritten. Older builds cannot read schema version 1, and previously overwritten customizations cannot be reconstructed by this fix. Issue 10 remains open.

- **Issue 9 committed and pushed:** `897adfc` on `origin/main`.
- **Issue 10 implemented (2026-11-12):** profile dirty state compares the editor with the exact last successfully saved snapshot, including undo/redo and edits during in-flight saves. Explicit profile writes are serialized and snapshot their arguments. Settings/profile flushes wait for work added while waiting. Legacy imports merge into the current editor state instead of replacing unsaved edits with backend state.
- Closing the native window now hides it to a system tray created before workers start, preserving the webview/editor session and ongoing deck control. Tray Quit restores the editor for an explicit Save and quit / Discard and quit / Cancel dialog. Save errors or changed revisions prevent exit; pending writes settle first. Quit cancels macros and waits for completion, refusing to force-kill an in-flight action; active/busy Game Mode must be ended first. Browser unload protection covers unsaved edits. Settings offers opt-in per-user Windows sign-in launch using the current executable and `--background`; no startup registration was enabled during this work.
- Validation: **91 Rust tests passed, 3 ignored; 8 frontend tests passed; production build and diff checks passed**. Added five frontend regressions for revision comparisons, in-flight edits, serialization/failure recovery and shutdown flushes, plus a Rust executable-path quoting test. Native tray/sign-in/window-reopen and packaged installer smoke tests remain pending. Hidden edits are retained in memory, not recovered after crashes or forced OS shutdown. All ten audited issues have code fixes; the physical-device and release acceptance gates below are still unverified.

## Follow-up improvements

- **Follow-up 1 implemented (2026-11-12): Clear key consistency.** Clear Key in Editor now changes only the undoable draft, removes its old media/image and action, and tells the user to Apply to Device. Removed the direct hardware-clear IPC command and bridge so a UI clear cannot blank the device while retaining its applied binding. Normal coordinated apply commits the empty image/action together; `none`/empty key bindings dispatch no hardware action.
- Added an isolated regression covering saved clear versus applied state, partial transfer failure, successful retry, unaffected neighboring bindings and reconnect recovery. Validation: **92 Rust tests passed, 3 ignored; 8 frontend tests passed; production build and diff checks passed**. Physical clear/apply testing remains pending.
- **Follow-up 1 committed and pushed:** `c47fb13` on `origin/main`.
- **Follow-up 2 implemented (2026-11-13): Action failure reporting.** Action execution now carries explicit success/failure through hardware diagnostics and the Test Action command; callers no longer guess outcomes from message text. Unsupported actions and missing navigation targets return errors. Brightness commits its stored value only after delivery. Macro start rejection and navigation failures propagate, including SwitchProfile macro steps. UI tests run off the command thread and display failed results/errors as warnings.
- Standalone webhooks reuse the owned-child runner with a six-second deadline and a shared single-request limit; HTTP errors and process failures propagate. Clipboard failures prevent pasting stale contents. Windows launches use ShellExecuteEx with error dialogs suppressed, allowing missing executables to return errors. Shell launches and asynchronous Game Mode requests explicitly report submission rather than completion.
- Added five regression tests for result semantics, invalid/unsupported actions, nonexistent executables, missing navigation/disconnected brightness, and actual loopback HTTP 200/500 responses plus overlapping request rejection. Validation: **97 Rust tests passed, 3 ignored; 8 frontend tests passed; production build and diff checks passed**. No external webhook endpoints were contacted. Physical integrations and interactive toast verification remain pending; successful OS dispatch cannot establish that the receiving application completed the requested operation. A standalone webhook can occupy the action worker for up to six seconds while HID I/O remains on its separate thread.
- **Follow-up 2 committed and pushed:** `2c901d6` on `origin/main`.
- **Follow-up 3 implemented (2026-11-13): Settings and quick-start UI.** Replaced eight mostly repeated Settings categories with four distinct sections: General, Devices & integrations, Data & backups, and About. Added contextual navigation to working screens, clear save/apply and backup limitations, desktop/browser distinction, and a version read from package metadata. Removed stale compiler versions, zero-polling claims, and unsupported freeze guarantees.
- The quick-start dialog now explains handoff, Save versus Apply, and tray behavior. Device text is based on the supplied connection list and explicitly identifies the browser preview; it never claims verification merely from opening the guide. Added dialog labelling, focus trapping/restoration, Escape dismissal, and suppressed editor shortcuts while the guide is open.
- Validation: **8 frontend tests passed; production build and diff checks passed**. Browser preview at 1280×720 verified all four Settings sections, navigation to plugin inventory, disabled desktop-only startup control, guide layout, Shift+Tab wrapping, Escape dismissal and focus restoration. No native startup setting or physical device was changed. Existing global header/footer crowding at this width is deferred to follow-up 4. Backend unchanged; Rust suite was not rerun for this UI-only change.
- Next: workspace editing/applied-state clarity and responsive header/footer, complete media-inclusive backups, and draft recovery.

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

## Follow-up 4: workspace status and responsive chrome (2026-11-13)

- Replaced the crowded 28px footer with two rows: editor/profile/page and saved state, then device connection and comfortably sized Resend/Pause controls. Removed CPU/RAM/version clutter. Header tabs and toolbar controls wrap without splitting labels; narrow workspace columns preserve the editor, and the deck canvas starts at its top so overflow remains scrollable.
- Replaced the local `deviceSynced` flag with a backend comparison against the last successfully transferred page, including profile/device identity and readiness. Saving alone cannot claim an apply. Paused/disconnected/failed transfers cannot claim a matching active device. Polls are serialized, late responses for old drafts are ignored, and failed reads clear stale success. Browser simulation is explicitly labeled.
- Validation: 99 Rust tests passed, 3 ignored; 8 frontend tests passed; production build and diff checks passed. Added regressions for changed draft contents, saving without applying, undo equality, and different profile/page IDs. Browser checks at 1280x720 and 1100x700 confirmed readable footer controls and edit/save/apply/pause/resume transitions. Physical hardware and packaged desktop smoke tests remain pending.

## Follow-up 5: profiles and media backup (2026-11-13)

- Added a separate version-1 profiles backup in Settings > Data & backups. It captures all editor profiles (including drafts), all macros, and deduplicated referenced key/touch/strip media with original image/GIF or generated sequence bytes, playback configuration and SHA-256 checksums. Existing lightweight profile share files stay compatible.
- Restore validates the envelope, references, IDs, resource limits, checksums and media decoding before publishing new media. Copies receive fresh media/profile/macro IDs; cross-profile and macro actions, encoder/touch actions and macro SwitchProfile steps are remapped by ID or name. Current profiles and applied hardware bindings are untouched. Imported profiles remain drafts until Save/Apply; imported macros use the existing save queue and visible failure/retry controls.
- Export/import decoding and disk I/O run off the device worker lock. Export filenames use exclusive creation, flushed writes and failure cleanup. File selection reports read/size errors. Limits: 96 MB JSON, 64 MB source media, 256 MB decoded media, 256 media assets. A disk failure during media restore may leave unused new media files; it cannot overwrite existing media or publish partial restored profiles.
- Scope is explicitly profiles/macros/referenced media, not a machine-wide configuration backup: auto-switch rules, OBS settings, Game Modes, startup preferences and unused library assets are excluded and listed in the UI.
- Validation: frontend build and 10 tests passed; Rust library suite 103 passed, 3 ignored. New tests cover full key/zone/strip restore, exact GIF/sequence byte and timing round trips across reload, corrupt/path-like IDs rejected before writes, missing references/future versions, cross-profile/macro remapping and repeated restores without collisions. Browser preview confirmed layout and honest desktop-only controls. Native file-picker/IPC and physical-device smoke tests remain pending; tests use temporary stores rather than user profiles.

## Follow-up 6: unsaved profile draft recovery (2026-11-13)

- Unsaved profile edits now receive a separate versioned recovery copy in profile-draft.json after a 500 ms pause. Draft writes snapshot their input, coalesce rapid changes, serialize against clears, and report failures with a retry control. Disk writes run on a blocking worker without holding device state. They never update saved profiles or the applied layout.
- Startup offers Restore draft / Discard draft before editor access. Restore remains unsaved, supports Undo back to loaded saved profiles, and warns when saved profile content changed since the draft began. Timestamp-only differences do not cause conflicts. Already-saved draft content is ignored. Corrupt/newer recovery files are preserved and automatic replacement is blocked until an explicit discard; read failures offer retry.
- Save/undo-to-saved schedules an atomic clear. Explicit quit stops scheduled recovery work, waits for older writes, and requires successful clearing before quitting so discarded drafts cannot return. Failed/cancelled quits resume recovery. Quit treats unresolved recovery as unsaved work; editor shortcuts are suspended during recovery/quit prompts.
- Validation: production build, 15 frontend tests and 106 Rust tests passed (3 ignored). New regressions cover input snapshots/coalescing, in-flight clear ordering, failure/retry, stale failures, cancelled quit, restart recovery, changed saved profiles, timestamp-only changes, interrupted writes, failed clears and corrupt/future stores. Browser preview verified fresh-session recovery, restored title with Unsaved status and no applied layout, and discard surviving reload. Tests used isolated stores; native forced-process-exit and OS shutdown smoke tests remain pending.
- Scope: profile/page/key/encoder/touch edits and their persisted media references. Macros/rules/OBS retain their existing save queues. Edits made before the debounce or disk write completes can still be lost on sudden termination; the editor displays recovery progress/errors. This is a latest-draft recovery copy, not version history or a full backup.


## Performance follow-up 1: Windows media polling blocked shared state (2026-11-14)

- The desktop worker sampled Windows media properties, album art, audio endpoints, system stats and plugin faces every two seconds while holding SharedState. Media operations synchronously wait on WinRT async results; a slow provider therefore also delayed UI commands and hardware action dispatch that needed state.
- Sampling now snapshots the previous face and consumes the existing refresh request under a short lock, releases state for OS queries, then locks only to publish the result. Refresh requests arriving during sampling remain pending. Unchanged samples still avoid unnecessary redraws, except when an explicit refresh/retry was requested.
- Validation: Rust library suite 128 passed, 7 ignored. A deterministic blocked-sampler regression proves another caller can acquire state while the query is waiting and schedule a refresh that survives publication. A second regression covers unchanged samples and explicit refreshes. No installed-app latency improvement has been measured yet; the running installed binary does not receive source changes automatically.
- Next performance candidates: get_telemetry synchronously enumerates all processes, requested by the frontend every five seconds; face_delta_bursts performs rendering while holding state. These need separate investigation/fixes. This change removes OS polling from the shared lock, not every possible source of delay.


## Performance follow-up 2: telemetry off the window thread (2026-11-14)

- get_telemetry now runs process enumeration and memory sampling on Tauri's blocking worker pool instead of the synchronous window command path. Join failures propagate through the existing IPC error path.
- Frontend telemetry polls schedule five seconds after the previous request settles. Slow scans cannot accumulate overlapping requests; cleanup cancels future polls and suppresses stale in-flight results. Failures retain the previous reading and retry normally.
- Validation: 20 frontend tests, production frontend build, and 128 Rust library tests passed (7 ignored). Polling regressions cover a request delayed across many intervals, result publication, disposal during a request, failure/retry and cancellation before the next read. Installed-app latency remains to be measured after installing a new build.


## Performance follow-up 3: live image rendering outside shared state (2026-11-14)

- Live face redraws now snapshot the applied page/device briefly, release SharedState for artwork decoding and JPEG encoding, reacquire it for cache planning, and build HID packets outside the lock. The deck-face UI event is also emitted after releasing state.
- DISPLAY_WRITE still spans snapshot, rendering, cache planning and transmission, so other display writes cannot replace the applied page or mix cache batches. Layout readiness and device connectivity are checked again before staging rendered frames. Failed writes retain the existing invalidation/retry behavior.
- Validation: Rust library suite 129 passed, 7 ignored. A new regression acquires state from inside the renderer and simulates a layout becoming unavailable, verifying obsolete frames never reach cache planning. Existing tests still verify rendering from the applied page after editor changes, successful-frame deduplication and retry after failed transfers.
- Scope: periodic live face redraws. Full layout preparation and animation packet planning remain separate performance candidates. No claim of measured installed-app latency improvement until a new build is installed and exercised.
