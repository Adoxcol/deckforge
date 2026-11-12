use super::*;
use std::sync::{atomic::{AtomicBool, Ordering}, OnceLock};
use std::time::Instant;

const MAX_STEPS: usize = 256;
const RUN_LIMIT: Duration = Duration::from_secs(60);

#[derive(Default)]
struct Runner { active: Mutex<Option<Arc<AtomicBool>>> }
static RUNNER: OnceLock<Arc<Runner>> = OnceLock::new();
fn runner() -> &'static Arc<Runner> { RUNNER.get_or_init(|| Arc::new(Runner::default())) }

pub(crate) struct RunPermit { owner: Arc<Runner>, control: RunControl }
struct RunControl { cancelled: Arc<AtomicBool>, deadline: Instant }
impl Drop for RunPermit {
    fn drop(&mut self) { *self.owner.active.lock_safe() = None; }
}
impl Runner {
    fn acquire(self: &Arc<Self>, steps: usize) -> Result<RunPermit, String> {
        if steps > MAX_STEPS { return Err(format!("A macro may contain at most {MAX_STEPS} steps")); }
        let mut active = self.active.lock_safe();
        if active.is_some() { return Err("A macro is already running. Wait or stop it before starting another.".into()); }
        let cancelled = Arc::new(AtomicBool::new(false));
        *active = Some(cancelled.clone());
        Ok(RunPermit { owner: self.clone(), control: RunControl { cancelled, deadline: Instant::now() + RUN_LIMIT } })
    }
    fn cancel(&self) -> bool {
        if let Some(active) = self.active.lock_safe().as_ref() { active.store(true, Ordering::Release); true } else { false }
    }
}
pub(crate) fn reserve_macro(seq: &db::MacroSequence) -> Result<RunPermit, String> { runner().acquire(seq.steps.len()) }
pub(crate) fn macro_is_running() -> bool { runner().active.lock_safe().is_some() }
pub(crate) fn cancel_running_macro() -> bool { runner().cancel() }
impl RunControl {
    fn check(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Acquire) { Err("Cancelled".into()) }
        else if Instant::now() >= self.deadline { Err("Timed out".into()) }
        else { Ok(()) }
    }
    fn wait(&self, duration: Duration) -> Result<(), String> {
        let until = Instant::now() + duration;
        loop {
            self.check()?;
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() { return Ok(()); }
            std::thread::sleep(left.min(Duration::from_millis(10)));
        }
    }
}

// Own the child until it exits. Cancellation and timeout kill and reap it.
fn wait_for_child(child: &mut std::process::Child, control: &RunControl) -> Result<(), String> {
    let result = (|| loop {
        control.check()?;
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return if status.success() { Ok(()) } else { Err(format!("Webhook process failed: {status}")) };
        }
        control.wait(Duration::from_millis(10))?;
    })();
    if result.is_err() { let _ = child.kill(); let _ = child.wait(); }
    result
}

fn run_webhook(endpoint: &str, control: &RunControl) -> Result<String, String> {
    let clean = endpoint.trim();
    if !(clean.starts_with("https://") || clean.starts_with("http://")) { return Err("Webhook requires an HTTP or HTTPS URL".into()); }
    control.check()?;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        let script = format!("try {{ Invoke-RestMethod -Uri '{}' -TimeoutSec 5 -ErrorAction Stop | Out-Null; exit 0 }} catch {{ exit 1 }}", clean.replace('\'', "''"));
        let mut child = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(0x08000000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn().map_err(|e| e.to_string())?;
        wait_for_child(&mut child, control)?;
        Ok("Webhook completed".into())
    }
    #[cfg(not(target_os = "windows"))]
    Err("Macro webhooks are currently supported on Windows only".into())
}


pub(crate) fn find_macro(guard: &ManagedAppState, macro_id: &str) -> Option<db::MacroSequence> {
    let needle = macro_id.trim();
    guard.macros.iter().find(|m| m.id == needle || m.name.eq_ignore_ascii_case(needle)).cloned()
}

fn run_one_macro_step(
    app: &tauri::AppHandle,
    shared: &SharedState,
    step: &db::MacroStep,
    control: &RunControl,
) -> (bool, String) {
    match step.step_type.as_str() {
        "Delay" => {
            let ms = macro_delay_ms(&step.parameter);
            match control.wait(Duration::from_millis(ms)) {
                Ok(()) => (true, format!("Waited {ms}ms")), Err(err) => (false, err)
            }
        }
        "Shortcut" => match send_hotkey_chord(&step.parameter) {
            Ok(msg) => (true, msg),
            Err(err) => (false, err),
        },
        "LaunchApp" => match launch_app_or_url(&step.parameter) {
            Ok(msg) => (true, msg),
            Err(err) => (false, err),
        },
        "HttpRequest" => match run_webhook(&step.parameter, control) {
            Ok(msg) => (true, msg),
            Err(err) => (false, err),
        },
        "IfWindowExists" => {
            let running = process_is_running(&step.parameter);
            if running {
                (true, format!("{} is running", step.parameter))
            } else {
                (false, format!("{} is not running", step.parameter))
            }
        }
        "SwitchProfile" => match apply_navigation(app, shared, "profile.switch", &step.parameter) {
            Some(msg) => (true, msg),
            None => (false, format!("No profile named {}", step.parameter)),
        },
        other => (false, format!("Unknown step {other}")),
    }
}

pub(crate) fn run_macro_sequence(
    app: &tauri::AppHandle,
    shared: &SharedState,
    seq: &db::MacroSequence,
    permit: RunPermit,
) -> MacroRunReport {
    let report = run_sequence_with(seq, &permit.control, |step, control| run_one_macro_step(app, shared, step, control));
    shared.lock_safe().diagnostics.push(DiagnosticEntry {
        id: format!("diag-macro-{}", now_ms()), timestamp_ms: now_ms(),
        level: if report.step_results.iter().any(|s| s.status != "Passed") { "warning" } else { "success" }.into(),
        device_id: "macro-runner".into(), stage: "MACRO_RUN".into(),
        message: format!("{}: {} ({} steps, {}ms)", report.macro_name,
            report.step_results.last().map(|s| s.status.as_str()).unwrap_or("No steps"),
            report.step_results.len(), report.total_elapsed_ms), packet_hex: None,
    });
    report
}

fn run_sequence_with(seq: &db::MacroSequence, control: &RunControl,
    mut execute: impl FnMut(&db::MacroStep, &RunControl) -> (bool, String)) -> MacroRunReport {
    use actions::MacroStepResult;
    let start = Instant::now();
    let mut step_results = Vec::with_capacity(seq.steps.len());
    for step in &seq.steps {
        let started = Instant::now();
        let step_control = RunControl { cancelled: control.cancelled.clone(),
            deadline: control.deadline.min(started + Duration::from_millis(step.timeout_ms.clamp(1, 60_000) as u64)) };
        let tries = if step.on_error == "RetryOnce" { 2 } else { 1 };
        let mut outcome = (false, String::new());
        let mut stopped = None;
        for _ in 0..tries {
            if let Err(reason) = step_control.check() { stopped = Some(reason); break; }
            outcome = execute(step, &step_control);
            if let Err(reason) = step_control.check() { stopped = Some(reason); break; }
            if outcome.0 { break; }
        }
        let terminal = stopped.is_some();
        let status = stopped.as_deref().unwrap_or(if outcome.0 { "Passed" } else { "Failed" }).to_string();
        step_results.push(MacroStepResult {
            step_id: step.id.clone(), label: step.label.clone(), step_type: step.step_type.clone(),
            status, elapsed_ms: started.elapsed().as_millis().max(1) as u64,
            detail: stopped.unwrap_or(outcome.1),
        });
        if terminal || (!outcome.0 && step.on_error != "Continue") { break; }
    }
    MacroRunReport { macro_id: seq.id.clone(), macro_name: seq.name.clone(),
        total_elapsed_ms: start.elapsed().as_millis().max(1) as u64, step_results }
}

pub(crate) fn start_macro(
    app: &tauri::AppHandle,
    shared: &SharedState,
    macro_id: &str,
    label: &str,
) -> String {
    let seq = Some(shared.lock_safe()).and_then(|guard| find_macro(&guard, macro_id));
    let Some(seq) = seq else {
        return format!("No macro named {macro_id}");
    };
    let permit = match reserve_macro(&seq) { Ok(permit) => permit, Err(err) => return err };
    let name = seq.name.clone();
    let app = app.clone();
    let shared = shared.clone();
    let spawned = std::thread::Builder::new().name("deckforge-macro".into()).spawn(move || {
        let report = run_macro_sequence(&app, &shared, &seq, permit);
        let _ = app.emit("macro-finished", &report);
    });
    if let Err(err) = spawned { return format!("Could not start macro: {err}"); }
    if label.is_empty() {
        format!("Started {name}")
    } else {
        format!("Started {label}")
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn sequence(policy: &str) -> db::MacroSequence {
        db::MacroSequence { id: "test".into(), name: "Test".into(), description: String::new(), last_edited_ms: 0,
            steps: (0..3).map(|n| db::MacroStep { id: n.to_string(), label: n.to_string(),
                step_type: "Delay".into(), parameter: "100".into(), timeout_ms: 500, on_error: policy.into() }).collect() }
    }

    #[test]
    fn simultaneous_starts_have_one_slot_and_cancel_does_not_release_it_early() {
        let owner = Arc::new(Runner::default());
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let handles: Vec<_> = (0..8).map(|_| {
            let owner = owner.clone(); let barrier = barrier.clone();
            std::thread::spawn(move || { let permit = owner.acquire(1).ok(); barrier.wait(); permit })
        }).collect();
        let permits: Vec<_> = handles.into_iter().filter_map(|h| h.join().unwrap()).collect();
        assert_eq!(permits.len(), 1);
        assert!(owner.cancel());
        assert!(owner.acquire(1).is_err());
        assert!(permits[0].control.check().is_err());
        drop(permits);
        assert!(!owner.cancel());
        assert!(owner.acquire(MAX_STEPS + 1).is_err());
        assert!(owner.acquire(1).is_ok());
    }

    #[test]
    fn cancellation_stops_remaining_steps_even_with_continue_or_retry() {
        for policy in ["Continue", "RetryOnce"] {
            let owner = Arc::new(Runner::default());
            let permit = owner.acquire(3).unwrap();
            let mut calls = 0;
            let report = run_sequence_with(&sequence(policy), &permit.control, |_, _| {
                calls += 1; owner.cancel(); (false, "failed".into())
            });
            assert_eq!(calls, 1);
            assert_eq!(report.step_results.len(), 1);
            assert_eq!(report.step_results[0].status, "Cancelled");
        }
    }

    #[test]
    fn cancelled_delay_wakes_promptly_and_expired_run_never_dispatches() {
        let owner = Arc::new(Runner::default());
        let mut permit = owner.acquire(3).unwrap();
        let cancelled = permit.control.cancelled.clone();
        let thread = std::thread::spawn(move || { std::thread::sleep(Duration::from_millis(20)); cancelled.store(true, Ordering::Release); });
        let started = Instant::now();
        assert!(permit.control.wait(Duration::from_secs(30)).is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
        thread.join().unwrap();
        permit.control.cancelled.store(false, Ordering::Release);
        permit.control.deadline = Instant::now();
        let report = run_sequence_with(&sequence("Continue"), &permit.control, |_, _| panic!("expired run dispatched"));
        assert_eq!(report.step_results[0].status, "Timed out");
    }

    #[test]
    fn retry_shares_step_deadline_and_timeout_overrides_continue() {
        for policy in ["Continue", "RetryOnce"] {
            let owner = Arc::new(Runner::default());
            let permit = owner.acquire(3).unwrap();
            let mut seq = sequence(policy); seq.steps[0].timeout_ms = 20;
            let mut calls = 0;
            let report = run_sequence_with(&seq, &permit.control, |_, control| {
                calls += 1;
                if policy == "Continue" || calls == 2 { let _ = control.wait(Duration::from_secs(10)); }
                (false, "failed".into())
            });
            assert_eq!(calls, if policy == "Continue" { 1 } else { 2 });
            assert_eq!(report.step_results.len(), 1);
            assert_eq!(report.step_results[0].status, "Timed out");
        }
    }

    #[test]
    fn ordinary_failures_obey_continue_stop_and_retry_once() {
        for (policy, expected) in [("Continue", 3), ("Stop", 1), ("RetryOnce", 2)] {
            let owner = Arc::new(Runner::default());
            let permit = owner.acquire(3).unwrap();
            let mut calls = 0;
            run_sequence_with(&sequence(policy), &permit.control, |_, _| { calls += 1; (false, "failed".into()) });
            assert_eq!(calls, expected);
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn slow_child_is_killed_and_reaped_on_cancel_or_timeout_and_failure_is_reported() {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        for cancel in [true, false] {
            let owner = Arc::new(Runner::default());
            let mut permit = owner.acquire(1).unwrap();
            let mut child = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 30"])
                .creation_flags(0x08000000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
            if cancel { owner.cancel(); } else { permit.control.deadline = Instant::now() + Duration::from_millis(30); }
            assert!(wait_for_child(&mut child, &permit.control).is_err());
            assert!(child.try_wait().unwrap().is_some());
        }
        let owner = Arc::new(Runner::default());
        let permit = owner.acquire(1).unwrap();
        let mut child = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", "exit 7"])
            .creation_flags(0x08000000).spawn().unwrap();
        assert!(wait_for_child(&mut child, &permit.control).unwrap_err().contains("failed"));
    }
}
