use super::*;

pub(crate) fn find_macro(guard: &ManagedAppState, macro_id: &str) -> Option<db::MacroSequence> {
    let needle = macro_id.trim();
    guard.macros.iter().find(|m| m.id == needle || m.name.eq_ignore_ascii_case(needle)).cloned()
}

pub(crate) fn run_one_macro_step(
    app: &tauri::AppHandle,
    shared: &SharedState,
    step: &db::MacroStep,
) -> (bool, String) {
    match step.step_type.as_str() {
        "Delay" => {
            let ms = macro_delay_ms(&step.parameter);
            std::thread::sleep(std::time::Duration::from_millis(ms));
            (true, format!("Waited {ms}ms"))
        }
        "Shortcut" => match send_hotkey_chord(&step.parameter) {
            Ok(msg) => (true, msg),
            Err(err) => (false, err),
        },
        "LaunchApp" => match launch_app_or_url(&step.parameter) {
            Ok(msg) => (true, msg),
            Err(err) => (false, err),
        },
        "HttpRequest" => match trigger_http_webhook(&step.parameter) {
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
) -> MacroRunReport {
    use actions::MacroStepResult;
    let start = std::time::Instant::now();
    let mut step_results = Vec::with_capacity(seq.steps.len());
    for step in &seq.steps {
        let started = std::time::Instant::now();
        let tries = if step.on_error == "RetryOnce" { 2 } else { 1 };
        let mut outcome = (false, String::new());
        for _ in 0..tries {
            outcome = run_one_macro_step(app, shared, step);
            if outcome.0 {
                break;
            }
        }
        let passed = outcome.0;
        step_results.push(MacroStepResult {
            step_id: step.id.clone(),
            label: step.label.clone(),
            step_type: step.step_type.clone(),
            status: if passed { "Passed" } else { "Failed" }.to_string(),
            elapsed_ms: started.elapsed().as_millis().max(1) as u64,
            detail: outcome.1,
        });
        if !passed && step.on_error != "Continue" {
            break;
        }
    }
    MacroRunReport {
        macro_id: seq.id.clone(),
        macro_name: seq.name.clone(),
        total_elapsed_ms: start.elapsed().as_millis().max(1) as u64,
        step_results,
    }
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
    let name = seq.name.clone();
    let app = app.clone();
    let shared = shared.clone();
    std::thread::spawn(move || {
        let report = run_macro_sequence(&app, &shared, &seq);
        { let mut guard = shared.lock_safe();
            guard.diagnostics.push(DiagnosticEntry {
                id: format!("diag-macro-{}", now_ms()),
                timestamp_ms: now_ms(),
                level: if report.step_results.iter().any(|step| step.status != "Passed") {
                    "warning"
                } else {
                    "success"
                }
                .to_string(),
                device_id: "macro-runner".to_string(),
                stage: "MACRO_RUN".to_string(),
                message: format!(
                    "Ran {} ({} steps, {}ms)",
                    report.macro_name,
                    report.step_results.len(),
                    report.total_elapsed_ms
                ),
                packet_hex: None,
            });
        }
        let _ = app.emit("macro-finished", &report);
    });
    if label.is_empty() {
        format!("Started {name}")
    } else {
        format!("Started {label}")
    }
}

