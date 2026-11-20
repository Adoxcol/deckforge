import type { Profile } from "../types/deckforge";

export interface MacroName {
  id: string;
  name: string;
}

export interface BindingHit {
  profileId: string;
  profileName: string;
  pageIndex: number;
  pageName: string;
  surface: "key" | "zone" | "encoder";
  index: number;
  title: string;
  where: string;
}

const CAP = 30;

function wordsOf(query: string): string[] {
  return query.toLowerCase().split(/\s+/).filter(Boolean);
}

function matches(hay: string, words: string[]): boolean {
  const lower = hay.toLowerCase();
  return words.every((word) => lower.includes(word));
}

function macroName(id: string, macros: MacroName[]): string {
  return macros.find((macro) => macro.id === id)?.name ?? "";
}

function actionText(actionId: string, param: string, label: string, macros: MacroName[]): string {
  const named = actionId === "macro.run" ? macroName(param, macros) : "";
  return `${label} ${actionId} ${param} ${named}`;
}

/** Finds keys, touch zones, and knobs whose name or action matches every word. */
export function searchBindings(profiles: Profile[], query: string, macros: MacroName[] = []): BindingHit[] {
  const words = wordsOf(query);
  if (words.length === 0) return [];
  const hits: BindingHit[] = [];
  for (const profile of profiles) {
    profile.pages.forEach((page, pageIndex) => {
      if (hits.length >= CAP) return;
      for (const key of page.keys) {
        if (hits.length >= CAP) return;
        const gestures = [key.doublePress, key.hold].filter((gesture) => gesture && gesture.actionId && gesture.actionId !== "none");
        const hay = [
          profile.name,
          page.name,
          key.title,
          key.subtitle,
          actionText(key.actionId, key.actionParam, key.actionLabel, macros),
          ...gestures.map((gesture) => actionText(gesture!.actionId, gesture!.actionParam, gesture!.actionLabel, macros)),
        ].join(" ");
        if (!matches(hay, words)) continue;
        const extra = key.doublePress?.actionId && key.doublePress.actionId !== "none" ? " · double-press" : "";
        const held = key.hold?.actionId && key.hold.actionId !== "none" ? " · hold" : "";
        hits.push({
          profileId: profile.id,
          profileName: profile.name,
          pageIndex,
          pageName: page.name,
          surface: "key",
          index: key.index,
          title: key.title || "Empty",
          where: `Key ${key.index + 1}${extra}${held}`,
        });
      }
      for (const zone of page.touchZones) {
        if (hits.length >= CAP) return;
        const hay = [profile.name, page.name, zone.label, zone.valueText, actionText(zone.tapActionId, zone.tapActionParam ?? "", "", macros)].join(" ");
        if (!matches(hay, words)) continue;
        hits.push({
          profileId: profile.id,
          profileName: profile.name,
          pageIndex,
          pageName: page.name,
          surface: "zone",
          index: zone.index,
          title: zone.label || "Zone",
          where: `Touch zone ${zone.index + 1}`,
        });
      }
      for (const enc of page.encoders) {
        if (hits.length >= CAP) return;
        const parts = [
          ["turn right", enc.cwAction, enc.cwParam],
          ["turn left", enc.ccwAction, enc.ccwParam],
          ["press", enc.pushAction, enc.pushParam],
        ] as const;
        const hay = [profile.name, page.name, enc.label, enc.valueText, ...parts.map(([, action, param]) => actionText(action, param ?? "", "", macros))].join(" ");
        if (!matches(hay, words)) continue;
        hits.push({
          profileId: profile.id,
          profileName: profile.name,
          pageIndex,
          pageName: page.name,
          surface: "encoder",
          index: enc.index,
          title: enc.label || "Knob",
          where: `Knob ${enc.index + 1}`,
        });
      }
    });
  }
  return hits;
}
