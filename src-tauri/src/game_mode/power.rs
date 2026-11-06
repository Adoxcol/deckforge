use super::*;

// ---------------------------------------------------------------------------
// Power plan
// ---------------------------------------------------------------------------
//
// Game Mode never assumes a plan exists. It lists the plans installed on this PC, lets each
// mode pick one of them (or none), and puts the previous plan back when the mode ends.

pub(super) fn powercfg(args: &[&str]) -> String {
    Command::new("powercfg")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

pub(super) fn guid_in(line: &str) -> Option<String> {
    line.split_whitespace()
        .map(|token| token.trim_matches(|c| c == '*' || c == '(' || c == ')'))
        .find(|token| is_guid(token))
        .map(|token| token.to_lowercase())
}

fn is_guid(text: &str) -> bool {
    text.len() == 36
        && text.matches('-').count() == 4
        && text.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// A GUID in canonical lowercase form, or an empty string for anything else.
pub(super) fn clean_guid(raw: &str) -> String {
    let text = raw.trim().trim_matches(|c| c == '{' || c == '}');
    if is_guid(text) {
        text.to_lowercase()
    } else {
        String::new()
    }
}

pub(super) fn plan_label(line: &str) -> String {
    match (line.find('('), line.rfind(')')) {
        (Some(start), Some(end)) if end > start => line[start + 1..end].trim().to_string(),
        _ => String::new(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Plan {
    pub(super) guid: String,
    pub(super) name: String,
    pub(super) active: bool,
}

/// Reads the output of `powercfg /list`. The active plan is the one marked with an asterisk.
pub(super) fn parse_plans(listing: &str) -> Vec<Plan> {
    listing
        .lines()
        .filter_map(|line| {
            let guid = guid_in(line)?;
            let name = plan_label(line);
            Some(Plan {
                name: if name.is_empty() { guid.clone() } else { name },
                guid,
                active: line.trim_end().ends_with('*'),
            })
        })
        .collect()
}

/// The power plans installed on this PC.
pub(super) fn installed_plans() -> Vec<Plan> {
    parse_plans(&powercfg(&["/list"]))
}

pub(super) fn active_plan() -> Option<(String, String)> {
    let out = powercfg(&["/getactivescheme"]);
    let line = out.lines().find(|l| guid_in(l).is_some())?;
    Some((guid_in(line)?, plan_label(line)))
}

/// Switches to the plan a mode asked for, if it is installed here. Returns the name of the plan
/// in use, the GUID to put back afterwards and the GUID that was switched to. The last two are
/// empty when nothing was changed.
pub(super) fn apply_plan(wanted: &str, notes: &mut Vec<String>) -> (String, String, String) {
    let current = active_plan();
    let current_name = current.as_ref().map(|(_, name)| name.clone()).unwrap_or_default();
    let wanted = clean_guid(wanted);
    if wanted.is_empty() {
        return (current_name, String::new(), String::new());
    }
    let Some(target) = installed_plans().into_iter().find(|p| p.guid == wanted) else {
        notes.push("The power plan chosen for this mode is not installed on this PC, so it was left alone.".to_string());
        return (current_name, String::new(), String::new());
    };
    let Some((previous, _)) = current else {
        notes.push("Could not read the current power plan, so it was left alone.".to_string());
        return (current_name, String::new(), String::new());
    };
    if previous == target.guid {
        return (target.name, String::new(), String::new());
    }
    let _ = powercfg(&["/setactive", &target.guid]);
    let switched = active_plan().map(|(guid, _)| guid == target.guid).unwrap_or(false);
    if !switched {
        notes.push(format!("Windows did not switch to the {} power plan.", target.name));
        return (current_name, String::new(), String::new());
    }
    notes.push(format!("Switched the power plan to {} (was {}).", target.name, current_name));
    (target.name, previous, target.guid)
}

/// Puts the previous plan back, unless the user already changed the plan by hand since Game
/// Mode switched it.
pub(super) fn restore_plan(previous: &str, applied: &str, notes: &mut Vec<String>) {
    let previous = clean_guid(previous);
    if previous.is_empty() {
        return;
    }
    let in_use = active_plan().map(|(guid, _)| guid).unwrap_or_default();
    if in_use != clean_guid(applied) {
        notes.push("The power plan was changed while Game Mode ran, so it was left as it is.".to_string());
        return;
    }
    let Some(plan) = installed_plans().into_iter().find(|p| p.guid == previous) else {
        notes.push("The earlier power plan is no longer installed, so the current one was kept.".to_string());
        return;
    };
    let _ = powercfg(&["/setactive", &plan.guid]);
    notes.push(format!("Power plan put back to {}.", plan.name));
}
