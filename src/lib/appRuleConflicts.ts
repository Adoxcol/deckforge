import type { AutoSwitchRule } from "../types/deckforge";

const processKey = (name: string) => name.trim().toLowerCase().replace(/\.exe$/, "");

/** Unrestricted rules for one executable have one destination. Preserve title-specific rules. */
export function assignUnrestrictedApp(rules: AutoSwitchRule[], processName: string, profileId: string, pageId: string): AutoSwitchRule[] {
  const matching = rules.filter(rule => !rule.windowTitleContains.trim() && Boolean(rule.targetPageId) === Boolean(pageId) && processKey(rule.processName) === processKey(processName));
  if (matching.length === 0) return [...rules, {
    id: `rule-${crypto.randomUUID()}`, processName, windowTitleContains: "", targetProfileId: profileId,
    targetPageId: pageId, priority: Math.min(255, Math.max(0, ...rules.map(rule => rule.priority)) + 1), enabled: true,
  }];
  const primary = [...matching].sort((a, b) => a.priority - b.priority)[0];
  return rules.map(rule => matching.some(match => match.id === rule.id)
    ? { ...rule, targetProfileId: profileId, targetPageId: pageId, enabled: rule.id === primary.id }
    : rule);
}

export function conflictingRule(rules: AutoSwitchRule[], rule: AutoSwitchRule): AutoSwitchRule | null {
  if (!rule.enabled || rule.windowTitleContains.trim()) return null;
  return [...rules].filter(other => other.enabled && !other.windowTitleContains.trim() && processKey(other.processName) === processKey(rule.processName))
    .sort((a, b) => a.priority - b.priority)[0] ?? null;
}
