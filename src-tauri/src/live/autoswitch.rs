use super::deps::*;

/// The profile and page to show for the app in front, or None to stay put.
/// The first enabled rule by priority wins. A rule naming a page switches even inside the
/// profile already showing; a rule without one (or whose page was deleted) opens page 1,
/// and only when that profile is not already showing.
pub(crate) fn auto_switch_target(
    rules: &[AutoSwitchRule],
    profiles: &[Profile],
    active_profile_id: &str,
    active_page_index: usize,
    exe: &str,
    title: &str,
    is_game: bool,
) -> Option<(String, usize)> {
    let mut rules: Vec<&AutoSwitchRule> = rules.iter()
        .filter(|rule| rule_matches(rule, exe, title, is_game))
        .collect();
    // A page rule is local to the active profile. If it matches, it wins over
    // a global profile rule for the same app. A specific game beats @game.
    rules.sort_by_key(|rule| (
        rule.target_page_id.is_empty() || rule.target_profile_id != active_profile_id,
        is_any_game(rule), rule.priority,
    ));
    for rule in &rules {
        if rule.target_page_id.is_empty() {
            if rule.target_profile_id == active_profile_id { return None; }
            if let Some(profile) = profiles.iter().find(|profile| profile.id == rule.target_profile_id) {
                // A matching page rule in the destination profile should be sent
                // with the profile switch, avoiding a second full layout transfer.
                let page_index = rules.iter()
                    .filter(|local| local.target_profile_id == profile.id && !local.target_page_id.is_empty())
                    .find_map(|local| profile.pages.iter().position(|page| page.id == local.target_page_id))
                    .unwrap_or(0);
                return Some((rule.target_profile_id.clone(), page_index));
            }
        } else if rule.target_profile_id == active_profile_id {
            if let Some(profile) = profiles.iter().find(|profile| profile.id == active_profile_id) {
                if let Some(index) = profile.pages.iter().position(|page| page.id == rule.target_page_id) {
                    return (index != active_page_index).then(|| (active_profile_id.to_string(), index));
                }
            }
        }
    }
    None
}

/// Process name of the rule that matches any full-screen game instead of one program.
pub(crate) const ANY_GAME: &str = "@game";

fn is_any_game(rule: &AutoSwitchRule) -> bool {
    rule.process_name.trim().eq_ignore_ascii_case(ANY_GAME)
}

pub(crate) fn rule_matches(rule: &AutoSwitchRule, exe: &str, title: &str, is_game: bool) -> bool {
    if !rule.enabled {
        return false;
    }
    if is_any_game(rule) {
        return is_game;
    }
    let want = rule.process_name.trim().to_lowercase();
    if want.is_empty() {
        return false;
    }
    let exe = exe.to_lowercase();
    let bare = want.trim_end_matches(".exe").to_string();
    let name_ok = exe == want || exe == bare || exe == format!("{bare}.exe");
    if !name_ok {
        return false;
    }
    let needle = rule.window_title_contains.trim();
    needle.is_empty() || title.to_lowercase().contains(&needle.to_lowercase())
}
