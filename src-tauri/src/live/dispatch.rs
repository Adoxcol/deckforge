use super::deps::*;
use super::layout::*;
use super::routing::*;
use super::sampling::note_latch;

pub(crate) fn dispatch_live_action(
    app: &tauri::AppHandle, shared: &SharedState, action: &LiveAction,
) -> ActionExecutionResult {
    let start = std::time::Instant::now();
    let outcome = if action.action_id == "macro.run" {
        Some(start_macro(app, shared, &action.param, &action.label))
    } else if action.action_id == "sys.game_mode" {
        Some(start_game_mode_toggle(app, shared, &action.param))
    } else if action.action_id.starts_with(crate::plugins::ACTION_PREFIX) {
        Some(crate::plugins::key_press(action.key_index))
    } else {
        apply_brightness_action(shared, &action.action_id)
            .or_else(|| apply_navigation(app, shared, &action.action_id, &action.param))
    };
    if let Some(outcome) = outcome {
        let result = actions::action_result(&action.action_id, &action.label, start, outcome);
        if result.success { note_latch(shared, action); }
        return result;
    }
    let obs = shared.lock_safe().obs.clone();
    let result = execute_key_action_live(&action.action_id, &action.label, &action.param, true, &obs);
    if result.success && face_changes(&action.action_id) { shared.lock_safe().sample_after_ms = now_ms().saturating_add(160); }
    if result.success { note_latch(shared, action); }
    result
}

/// Nudges the key faces so the Game Mode key flips to GAME ON or back quickly.
pub(crate) fn nudge_face_sample(shared: &SharedState) {
    { let mut guard = shared.lock_safe();
        guard.sample_after_ms = now_ms().saturating_add(120);
    }
}

/// Game Mode closes apps one at a time, so it never runs on the input thread. The key's
/// parameter picks the mode by id; an empty one uses the mode chosen in the Game Mode tab.
pub(crate) fn start_game_mode_toggle(app: &tauri::AppHandle, shared: &SharedState, mode: &str) -> Result<String, String> {
    if game_mode::is_busy() {
        return Err("Game Mode is still working".to_string());
    }
    let turning_on = !game_mode::is_active();
    let app = app.clone();
    let shared_for_thread = Arc::clone(shared);
    let mode = mode.trim().to_string();
    std::thread::Builder::new().name("deckforge-game-mode-action".into()).spawn(move || {
        let message = game_mode::toggle(&app, Some(mode.as_str()).filter(|m| !m.is_empty()));
        nudge_face_sample(&shared_for_thread);
        let _ = app.emit("game-mode-message", &message);
    }).map_err(|err| format!("Could not start Game Mode action: {err}"))?;
    nudge_face_sample(shared);
    if turning_on {
        Ok("Game Mode start requested; follow progress in Game Mode".into())
    } else {
        Ok("Game Mode stop requested; follow progress in Game Mode".into())
    }
}

pub(crate) fn apply_navigation(
    app: &tauri::AppHandle,
    shared: &SharedState,
    action_id: &str,
    param: &str,
) -> Option<Result<String, String>> {
    if !matches!(
        action_id,
        "profile.next_page" | "profile.prev_page" | "profile.switch_page" | "profile.switch" | "profile.back"
    ) {
        return None;
    }
    let _display = DISPLAY_WRITE.lock_safe();
    let event = {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        let Some(message) = switch_active_page(&mut guard, action_id, param) else {
            return Some(Err(format!("No matching profile or page for `{param}`")));
        };
        DeckContextEvent {
            profile_id: guard.active_profile_id.clone(),
            page_index: guard.active_page_index,
            message,
        }
    };
    if let Err(err) = push_active_layout_locked(shared, false) {
        return Some(Err(format!("Page switch failed: {err}")));
    }
    let _ = app.emit("deck-context", &event);
    Some(Ok(event.message))
}

pub(crate) fn switch_active_page(guard: &mut ManagedAppState, action_id: &str, param: &str) -> Option<String> {
    let before = (guard.active_profile_id.clone(), guard.active_page_index);
    if action_id == "profile.back" {
        let (profile_id, page_index) = guard.previous_page.take()?;
        let profile = guard.profiles.iter().find(|profile| profile.id == profile_id)?;
        let page_index = page_index.min(profile.pages.len().saturating_sub(1));
        let page_name = profile.pages.get(page_index).map(|page| page.name.clone()).unwrap_or_default();
        guard.active_profile_id = profile_id;
        guard.active_page_index = page_index;
        guard.previous_page = Some(before);
        return Some(format!("Back to {page_name}"));
    }
    let message = switch_active_page_inner(guard, action_id, param)?;
    if (guard.active_profile_id.clone(), guard.active_page_index) != before {
        guard.previous_page = Some(before);
    }
    Some(message)
}

fn switch_active_page_inner(guard: &mut ManagedAppState, action_id: &str, param: &str) -> Option<String> {
    match action_id {
        "profile.switch" => {
            let profile = guard
                .profiles
                .iter()
                .find(|profile| {
                    profile.id.eq_ignore_ascii_case(param.trim())
                        || profile.name.eq_ignore_ascii_case(param.trim())
                })?
                .clone();
            let name = profile.name.clone();
            guard.active_profile_id = profile.id;
            guard.active_page_index = 0;
            Some(format!("Switched to {name}"))
        }
        "profile.next_page" | "profile.prev_page" | "profile.switch_page" => {
            let pages = guard
                .profiles
                .iter()
                .find(|profile| profile.id == guard.active_profile_id)
                .or_else(|| guard.profiles.first())?
                .pages
                .len();
            if pages == 0 {
                return None;
            }
            let next = match action_id {
                "profile.next_page" => (guard.active_page_index + 1) % pages,
                "profile.prev_page" => guard.active_page_index.checked_sub(1).unwrap_or(pages - 1),
                _ => resolve_page_index(guard, param)?,
            };
            guard.active_page_index = next;
            let page_name = guard
                .profiles
                .iter()
                .find(|profile| profile.id == guard.active_profile_id)
                .and_then(|profile| profile.pages.get(next))
                .map(|page| page.name.clone())
                .unwrap_or_else(|| format!("Page {}", next + 1));
            Some(format!("Opened {page_name}"))
        }
        _ => None,
    }
}

pub(crate) fn resolve_page_index(guard: &ManagedAppState, param: &str) -> Option<usize> {
    let param = param.trim();
    if param.is_empty() {
        return None;
    }
    let profile = guard
        .profiles
        .iter()
        .find(|profile| profile.id == guard.active_profile_id)
        .or_else(|| guard.profiles.first())?;
    if let Ok(number) = param.parse::<usize>() {
        if (1..=profile.pages.len()).contains(&number) {
            return Some(number - 1);
        }
    }
    let lower = param.to_lowercase();
    profile.pages.iter().position(|page| {
        page.id.eq_ignore_ascii_case(param)
            || page.name.eq_ignore_ascii_case(param)
            || page.name.to_lowercase().starts_with(&lower)
    })
}
