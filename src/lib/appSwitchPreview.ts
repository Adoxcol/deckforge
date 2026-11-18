import type { AutoSwitchRule, Profile } from "../types/deckforge";

export interface AppSwitchPreview {
  profileId: string;
  pageIndex: number;
  ruleId: string | null;
  pageRuleId: string | null;
  switches: boolean;
  reason: string;
}

const anyGame = (rule: AutoSwitchRule) => rule.processName.trim().toLowerCase() === "@game";

const matches = (rule: AutoSwitchRule, exe: string, title: string, isGame: boolean) => {
  if (!rule.enabled) return false;
  if (anyGame(rule)) return isGame;
  const want = rule.processName.trim().toLowerCase();
  if (!want) return false;
  const bare = want.endsWith(".exe") ? want.slice(0, -4) : want;
  const name = exe.toLowerCase();
  if (name !== want && name !== bare && name !== `${bare}.exe`) return false;
  return !rule.windowTitleContains.trim() || title.toLowerCase().includes(rule.windowTitleContains.trim().toLowerCase());
};

/** Mirrors the rule order and destination selection in Rust's auto_switch_target. */
export function previewAppSwitch(
  rules: AutoSwitchRule[], profiles: Profile[], activeProfileId: string, activePageIndex: number,
  exe: string, title: string, isGame: boolean,
): AppSwitchPreview {
  const stay = (ruleId: string | null, reason: string, pageRuleId: string | null = null): AppSwitchPreview =>
    ({ profileId: activeProfileId, pageIndex: activePageIndex, ruleId, pageRuleId, switches: false, reason });
  const matching = rules.filter(rule => matches(rule, exe.trim(), title, isGame)).sort((a, b) => {
    const scope = (rule: AutoSwitchRule) => Number(!rule.targetPageId || rule.targetProfileId !== activeProfileId);
    return scope(a) - scope(b) || Number(anyGame(a)) - Number(anyGame(b)) || a.priority - b.priority;
  });
  if (!matching.length) return stay(null, "No enabled rule matches this app and window title.");
  for (const rule of matching) {
    if (!rule.targetPageId) {
      if (rule.targetProfileId === activeProfileId) return stay(rule.id, "This profile is already active; a profile rule keeps its current page.");
      const profile = profiles.find(p => p.id === rule.targetProfileId);
      if (!profile) continue;
      const local = matching.map(candidate => ({ candidate, index: profile.pages.findIndex(page => page.id === candidate.targetPageId) }))
        .find(({ candidate, index }) => candidate.targetProfileId === profile.id && !!candidate.targetPageId && index >= 0);
      return {
        profileId: profile.id, pageIndex: local?.index ?? 0, ruleId: rule.id,
        pageRuleId: local?.candidate.id ?? null, switches: true,
        reason: local ? "Profile rule opens the destination profile; its matching page rule selects the page."
          : "Profile rule opens the destination profile on page 1.",
      };
    }
    if (rule.targetProfileId === activeProfileId) {
      const pageIndex = profiles.find(p => p.id === activeProfileId)?.pages.findIndex(page => page.id === rule.targetPageId) ?? -1;
      if (pageIndex < 0) continue;
      if (pageIndex === activePageIndex) return stay(rule.id, "The matching page is already showing.");
      return { profileId: activeProfileId, pageIndex, ruleId: rule.id, pageRuleId: null, switches: true,
        reason: "A page rule in the current profile takes priority over profile rules." };
    }
  }
  return stay(null, "Matching page rules belong to another profile or point to a deleted page.");
}
