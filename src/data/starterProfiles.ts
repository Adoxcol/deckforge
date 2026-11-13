import type { AutoSwitchRule, Profile } from "../types/deckforge";
import { BUILT_IN_TEMPLATES, instantiateTemplate } from "./pageTemplates";

/** A ready-made profile: pages from the page templates, plus the apps that open each page. */
export interface StarterProfile {
  id: string;
  name: string;
  tagline: string;
  icon: string;
  color: string;
  /** Page template id and the apps (exe names) that open that page. */
  pages: Array<{ template: string; name: string; apps: string[] }>;
}

export const STARTER_PROFILES: StarterProfile[] = [
  {
    id: "blank",
    name: "Blank",
    tagline: "One empty page.",
    icon: "Sparkles",
    color: "#707B94",
    pages: [{ template: "tpl-blank", name: "Main", apps: [] }],
  },
  {
    id: "streaming",
    name: "Streaming",
    tagline: "Go live in OBS, music on a second page.",
    icon: "Radio",
    color: "#F16B79",
    pages: [
      { template: "tpl-go-live", name: "Go Live", apps: ["obs64.exe"] },
      { template: "tpl-music", name: "Music", apps: ["spotify.exe"] },
    ],
  },
  {
    id: "editing",
    name: "Video editing",
    tagline: "Edit keys for Resolve and Premiere.",
    icon: "Film",
    color: "#F4B860",
    pages: [
      { template: "tpl-edit", name: "Edit", apps: ["resolve.exe", "premiere pro.exe"] },
      { template: "tpl-music", name: "Music", apps: [] },
    ],
  },
  {
    id: "coding",
    name: "Coding",
    tagline: "Dev keys for VS Code, links for the browser.",
    icon: "Terminal",
    color: "#35C98B",
    pages: [
      { template: "tpl-dev", name: "Dev", apps: ["code.exe"] },
      { template: "tpl-links", name: "Links", apps: [] },
    ],
  },
  {
    id: "meetings",
    name: "Meetings",
    tagline: "Mute and camera keys for Teams and Zoom.",
    icon: "Video",
    color: "#56B9F2",
    pages: [
      { template: "tpl-meeting", name: "Meeting", apps: ["ms-teams.exe", "teams.exe", "zoom.exe"] },
      { template: "tpl-office", name: "Office", apps: [] },
    ],
  },
  {
    id: "gaming",
    name: "Gaming",
    tagline: "Game keys for Steam, music on the side.",
    icon: "Gamepad2",
    color: "#8B5CF6",
    pages: [
      { template: "tpl-game", name: "Game", apps: ["@game", "steam.exe"] },
      { template: "tpl-music", name: "Music", apps: ["spotify.exe"] },
    ],
  },
];

/**
 * Builds a new profile from a starter, plus app rules for it. Apps that already have a rule
 * are skipped, so a starter never fights a rule the user set up.
 */
export function buildStarterProfile(
  starter: StarterProfile,
  existingProfiles: Profile[],
  existingRules: AutoSwitchRule[]
): { profile: Profile; rules: AutoSwitchRule[]; skippedApps: string[] } {
  const stamp = Date.now();
  const id = `profile-${starter.id}-${stamp}`;
  const taken = new Set(existingProfiles.map((p) => p.name.toLowerCase()));
  let name = starter.name;
  for (let n = 2; taken.has(name.toLowerCase()); n++) name = `${starter.name} ${n}`;

  const pages = starter.pages.map((spec, i) => {
    const template = BUILT_IN_TEMPLATES.find((t) => t.id === spec.template) ?? BUILT_IN_TEMPLATES[BUILT_IN_TEMPLATES.length - 1];
    return instantiateTemplate(template, spec.name, `page-${starter.id}-${stamp}-${i}`);
  });

  const used = new Set(existingRules.map((r) => r.processName.trim().toLowerCase()));
  const rules: AutoSwitchRule[] = [];
  const skippedApps: string[] = [];
  starter.pages.forEach((spec, i) =>
    spec.apps.forEach((exe) => {
      if (used.has(exe)) {
        skippedApps.push(exe);
        return;
      }
      used.add(exe);
      rules.push({
        id: `rule-${stamp}-${rules.length}`,
        processName: exe,
        windowTitleContains: "",
        targetProfileId: id,
        targetPageId: pages[i].id,
        priority: existingRules.length + rules.length + 1,
        enabled: true,
      });
    })
  );

  return {
    profile: { id, name, icon: starter.icon, colorStrip: starter.color, targetDeviceId: "", pages, lastModifiedMs: stamp },
    rules,
    skippedApps,
  };
}
