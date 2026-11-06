/**
 * Share files: export a profile or a Game Mode to a JSON file and read one back.
 *
 * Everything that comes out of a file is rebuilt field by field, so a hand-edited or damaged
 * file cannot put odd values into the editor. Pictures and animations are left out of exports
 * because they live in this PC's media library and would not exist on another PC.
 */
import type {
  EncoderConfig,
  GameModeCustomTarget,
  GameModeMode,
  KeyConfig,
  MacroSequence,
  MacroStep,
  Profile,
  ProfilePage,
  TouchZoneConfig,
} from "../types/deckforge";

export const PROFILE_FILE_KIND = "deckforge.profile";
export const MODE_FILE_KIND = "deckforge.game-mode";
const FORMAT_VERSION = 1;

type Rec = Record<string, unknown>;

const isRec = (value: unknown): value is Rec => typeof value === "object" && value !== null && !Array.isArray(value);
const str = (value: unknown, fallback = ""): string => (typeof value === "string" ? value : fallback);
const num = (value: unknown, fallback: number): number => (typeof value === "number" && Number.isFinite(value) ? value : fallback);
const bool = (value: unknown, fallback: boolean): boolean => (typeof value === "boolean" ? value : fallback);
const color = (value: unknown, fallback: string): string =>
  typeof value === "string" && /^#[0-9a-fA-F]{6}$/.test(value) ? value : fallback;
const oneOf = <T extends string>(value: unknown, allowed: readonly T[], fallback: T): T =>
  typeof value === "string" && (allowed as readonly string[]).includes(value) ? (value as T) : fallback;
const strings = (value: unknown): string[] => (Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : []);
const records = (value: unknown): Rec[] => (Array.isArray(value) ? value.filter(isRec) : []);

export function slugify(text: string, max = 40): string {
  return (
    text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, max) || "export"
  );
}

/** Opens the file chooser and reads the picked JSON file. Resolves null if the user cancels. */
export function pickJsonFile(): Promise<{ name: string; text: string } | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json,application/json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return resolve(null);
      resolve({ name: file.name, text: await file.text() });
    };
    input.addEventListener("cancel", () => resolve(null));
    input.click();
  });
}

function parseEnvelope(text: string, kind: string, what: string): Rec {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    throw new Error("That file is not valid JSON.");
  }
  if (!isRec(data) || data.kind !== kind) {
    throw new Error(`That file is not a DeckForge ${what}.`);
  }
  if (num(data.version, 1) > FORMAT_VERSION) {
    throw new Error(`That ${what} was made by a newer DeckForge. Update DeckForge and try again.`);
  }
  return data;
}

// ---------------------------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------------------------

export interface ProfileFile {
  kind: typeof PROFILE_FILE_KIND;
  version: number;
  exportedAt: string;
  profile: Profile;
  macros: MacroSequence[];
}

/** Calls `fn` for every action slot of a profile and writes back what it returns. */
export function rewriteActions(
  profile: Profile,
  fn: (actionId: string, param: string) => { actionId: string; param: string }
): Profile {
  return {
    ...profile,
    pages: profile.pages.map((page) => ({
      ...page,
      keys: page.keys.map((key) => {
        const next = fn(key.actionId, key.actionParam);
        return { ...key, actionId: next.actionId, actionParam: next.param };
      }),
      touchZones: page.touchZones.map((zone) => {
        const next = fn(zone.tapActionId, zone.tapActionParam ?? "");
        return { ...zone, tapActionId: next.actionId, tapActionParam: next.param };
      }),
      encoders: page.encoders.map((enc) => {
        const cw = fn(enc.cwAction, enc.cwParam ?? "");
        const ccw = fn(enc.ccwAction, enc.ccwParam ?? "");
        const push = fn(enc.pushAction, enc.pushParam ?? "");
        return {
          ...enc,
          cwAction: cw.actionId,
          cwParam: cw.param,
          ccwAction: ccw.actionId,
          ccwParam: ccw.param,
          pushAction: push.actionId,
          pushParam: push.param,
        };
      }),
    })),
  };
}

/** Ids of the macros a profile runs. */
export function macroIdsUsed(profile: Profile): string[] {
  const ids = new Set<string>();
  rewriteActions(profile, (actionId, param) => {
    if (actionId === "macro.run" && param) ids.add(param);
    return { actionId, param };
  });
  return [...ids];
}

export function buildProfileFile(profile: Profile, macros: MacroSequence[]): { file: ProfileFile; mediaRemoved: number } {
  let mediaRemoved = 0;
  const strip = <T extends { media?: unknown }>(item: T): T => {
    if (item.media) mediaRemoved += 1;
    return { ...item, media: null };
  };
  const clean: Profile = {
    ...profile,
    pages: profile.pages.map((page) => {
      if (page.stripMedia) mediaRemoved += 1;
      return {
        ...page,
        stripMedia: null,
        keys: page.keys.map(strip),
        touchZones: page.touchZones.map(strip),
      };
    }),
  };
  const used = new Set(macroIdsUsed(clean));
  return {
    mediaRemoved,
    file: {
      kind: PROFILE_FILE_KIND,
      version: FORMAT_VERSION,
      exportedAt: new Date().toISOString(),
      profile: clean,
      macros: macros.filter((macro) => used.has(macro.id)),
    },
  };
}

const TITLE_SIZES = ["Small", "Medium", "Large"] as const;
const BACKGROUNDS = ["Gradient", "Solid", "Image"] as const;
function readKey(raw: Rec, fallbackIndex: number): KeyConfig {
  const image = str(raw.customImageDataUrl);
  return {
    index: Math.max(0, Math.floor(num(raw.index, fallbackIndex))),
    title: str(raw.title).slice(0, 40),
    subtitle: str(raw.subtitle).slice(0, 40),
    iconName: str(raw.iconName, "LayoutGrid"),
    customImageDataUrl: image.startsWith("data:image/") ? image : undefined,
    media: null,
    actionId: str(raw.actionId, "none"),
    actionLabel: str(raw.actionLabel, "No action"),
    actionCategory: str(raw.actionCategory, "Custom"),
    actionParam: str(raw.actionParam),
    accentColor: color(raw.accentColor, "#8B5CF6"),
    bgColor: color(raw.bgColor, "#231B38"),
    showTitle: bool(raw.showTitle, true),
    titleSize: oneOf(raw.titleSize, TITLE_SIZES, "Medium"),
    backgroundStyle: oneOf(raw.backgroundStyle, BACKGROUNDS, "Gradient"),
    cornerRadius: Math.min(40, Math.max(0, num(raw.cornerRadius, 12))),
  };
}

function readZone(raw: Rec, fallbackIndex: number): TouchZoneConfig {
  return {
    index: Math.max(0, Math.floor(num(raw.index, fallbackIndex))),
    label: str(raw.label).slice(0, 40),
    valueText: str(raw.valueText).slice(0, 40),
    tapActionId: str(raw.tapActionId, "none"),
    tapActionParam: str(raw.tapActionParam),
    accentColor: color(raw.accentColor, "#8B5CF6"),
    media: null,
  };
}

function readEncoder(raw: Rec, fallbackIndex: number): EncoderConfig {
  return {
    index: Math.max(0, Math.floor(num(raw.index, fallbackIndex))),
    label: str(raw.label).slice(0, 40),
    valueText: str(raw.valueText).slice(0, 40),
    cwAction: str(raw.cwAction, "none"),
    ccwAction: str(raw.ccwAction, "none"),
    pushAction: str(raw.pushAction, "none"),
    cwParam: str(raw.cwParam),
    ccwParam: str(raw.ccwParam),
    pushParam: str(raw.pushParam),
  };
}

function readPage(raw: Rec, position: number): ProfilePage {
  return {
    id: str(raw.id) || `page-${position + 1}`,
    name: str(raw.name, `Page ${position + 1}`).slice(0, 40) || `Page ${position + 1}`,
    keys: records(raw.keys).map(readKey),
    touchZones: records(raw.touchZones).map(readZone),
    encoders: records(raw.encoders).map(readEncoder),
    stripMedia: null,
  };
}

const STEP_TYPES = ["LaunchApp", "Delay", "Shortcut", "IfWindowExists", "HttpRequest", "SwitchProfile"] as const;
const ON_ERROR = ["Stop", "Continue", "RetryOnce"] as const;

function readMacro(raw: Rec): MacroSequence | null {
  const id = str(raw.id);
  if (!id) return null;
  const steps: MacroStep[] = records(raw.steps).map((step, i) => ({
    id: str(step.id) || `${id}-step-${i + 1}`,
    stepType: oneOf(step.stepType, STEP_TYPES, "Delay"),
    label: str(step.label).slice(0, 80),
    parameter: str(step.parameter),
    timeoutMs: Math.min(600000, Math.max(0, num(step.timeoutMs, 5000))),
    onError: oneOf(step.onError, ON_ERROR, "Stop"),
  }));
  return {
    id,
    name: str(raw.name, id).slice(0, 60),
    description: str(raw.description),
    steps,
    lastEditedMs: Date.now(),
  };
}

export interface ParsedProfileFile {
  profile: Profile;
  macros: MacroSequence[];
}

export function parseProfileFile(text: string): ParsedProfileFile {
  const data = parseEnvelope(text, PROFILE_FILE_KIND, "profile file");
  const raw = data.profile;
  if (!isRec(raw)) throw new Error("The profile file has no profile in it.");
  const pages = records(raw.pages).map(readPage);
  if (pages.length === 0) throw new Error("The profile file has no pages.");
  const seen = new Set<string>();
  for (const page of pages) {
    let id = page.id;
    let n = 2;
    while (seen.has(id)) id = `${page.id}-${n++}`;
    page.id = id;
    seen.add(id);
  }
  const profile: Profile = {
    id: str(raw.id, "profile"),
    name: str(raw.name, "Imported profile").slice(0, 40) || "Imported profile",
    icon: str(raw.icon, "Sparkles"),
    colorStrip: color(raw.colorStrip, "#8B5CF6"),
    targetDeviceId: str(raw.targetDeviceId),
    pages,
    lastModifiedMs: Date.now(),
  };
  const macros = records(data.macros)
    .map(readMacro)
    .filter((macro): macro is MacroSequence => macro !== null);
  return { profile, macros };
}

/** Result of placing an imported profile next to the ones that already exist. */
export interface PlacedProfile {
  profile: Profile;
  macros: MacroSequence[];
  addedMacros: number;
}

/**
 * Gives an imported profile a fresh id and a free name, keeps its macros (renaming a macro
 * that clashes with a different one already here) and repoints keys that referenced either.
 */
export function placeImportedProfile(
  parsed: ParsedProfileFile,
  existingProfiles: Profile[],
  existingMacros: MacroSequence[]
): PlacedProfile {
  const stamp = Date.now().toString(36);
  const takenIds = new Set(existingProfiles.map((p) => p.id));
  let id = `profile-shared-${slugify(parsed.profile.name, 20)}-${stamp}`;
  for (let n = 2; takenIds.has(id); n++) id = `profile-shared-${slugify(parsed.profile.name, 20)}-${stamp}-${n}`;

  const takenNames = new Set(existingProfiles.map((p) => p.name.toLowerCase()));
  let name = parsed.profile.name;
  if (takenNames.has(name.toLowerCase())) {
    name = `${parsed.profile.name} (imported)`;
    for (let n = 2; takenNames.has(name.toLowerCase()); n++) name = `${parsed.profile.name} (imported ${n})`;
  }

  const macroRemap = new Map<string, string>();
  const nextMacros = [...existingMacros];
  let addedMacros = 0;
  for (const macro of parsed.macros) {
    const same = nextMacros.find((m) => m.id === macro.id);
    if (same && JSON.stringify(same.steps) === JSON.stringify(macro.steps)) continue;
    let macroId = macro.id;
    if (same) {
      for (let n = 2; nextMacros.some((m) => m.id === macroId); n++) macroId = `${macro.id}-${n}`;
      macroRemap.set(macro.id, macroId);
    }
    nextMacros.push({ ...macro, id: macroId, name: same ? `${macro.name} (imported)` : macro.name });
    addedMacros += 1;
  }

  const oldId = parsed.profile.id;
  const profile = rewriteActions({ ...parsed.profile, id, name }, (actionId, param) => {
    if (actionId === "profile.switch" && param === oldId) return { actionId, param: id };
    if (actionId === "macro.run" && macroRemap.has(param)) return { actionId, param: macroRemap.get(param) ?? param };
    return { actionId, param };
  });
  return { profile, macros: nextMacros, addedMacros };
}

// ---------------------------------------------------------------------------------------------
// Game Modes
// ---------------------------------------------------------------------------------------------

export interface ModeFile {
  kind: typeof MODE_FILE_KIND;
  version: number;
  exportedAt: string;
  mode: { name: string; targets: string[]; tweaks: string[]; powerPlan: string };
  customTargets: GameModeCustomTarget[];
}

export function buildModeFile(mode: GameModeMode, customTargets: GameModeCustomTarget[]): ModeFile {
  return {
    kind: MODE_FILE_KIND,
    version: FORMAT_VERSION,
    exportedAt: new Date().toISOString(),
    mode: { name: mode.name, targets: mode.targets, tweaks: mode.tweaks, powerPlan: mode.powerPlan },
    customTargets: customTargets.filter((target) => mode.targets.includes(target.id)),
  };
}

export interface ParsedModeFile {
  name: string;
  targets: string[];
  tweaks: string[];
  /** GUID of a power plan. Only used when the same plan is installed on this PC. */
  powerPlan: string;
  customTargets: GameModeCustomTarget[];
}

const GUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export function parseModeFile(text: string): ParsedModeFile {
  const data = parseEnvelope(text, MODE_FILE_KIND, "Game Mode file");
  const raw = data.mode;
  if (!isRec(raw)) throw new Error("The Game Mode file has no mode in it.");
  const customTargets: GameModeCustomTarget[] = records(data.customTargets)
    .map((item) => ({
      id: str(item.id),
      label: str(item.label).slice(0, 60),
      group: oneOf(item.group, ["apps", "background"] as const, "apps"),
      processes: strings(item.processes).slice(0, 40),
      services: strings(item.services).slice(0, 40),
      graceful: bool(item.graceful, true),
      reopen: bool(item.reopen, false),
    }))
    .filter((item) => item.id !== "" && item.label !== "" && (item.processes.length > 0 || item.services.length > 0));
  return {
    name: str(raw.name, "Imported mode").slice(0, 40) || "Imported mode",
    targets: strings(raw.targets),
    tweaks: strings(raw.tweaks),
    powerPlan: str(raw.powerPlan).trim().replace(/^\{|\}$/g, "").toLowerCase(),
    customTargets,
  };
}

export interface PlacedMode {
  mode: GameModeMode;
  customTargets: GameModeCustomTarget[];
  /** Targets or tweaks this build does not know, so they were left out. */
  dropped: number;
  /** The file asked for a power plan that is not installed on this PC, so the mode leaves it alone. */
  powerPlanDropped: boolean;
}

/**
 * Adds an imported mode to the current setup. Built-in targets and tweaks must exist in this
 * build. A custom target that clashes with a different one gets a new id.
 */
export function placeImportedMode(
  parsed: ParsedModeFile,
  existingModes: GameModeMode[],
  existingCustom: GameModeCustomTarget[],
  knownTargetIds: string[],
  knownTweakIds: string[],
  installedPlanIds: string[]
): PlacedMode {
  const builtIn = new Set(knownTargetIds.filter((id) => !existingCustom.some((c) => c.id === id)));
  const taken = new Set(knownTargetIds);
  const remap = new Map<string, string>();
  const customTargets = [...existingCustom];
  const sameTarget = (a: GameModeCustomTarget, b: GameModeCustomTarget) =>
    JSON.stringify([a.processes, a.services, a.group]) === JSON.stringify([b.processes, b.services, b.group]);

  for (const incoming of parsed.customTargets) {
    if (builtIn.has(incoming.id)) continue;
    const existing = customTargets.find((c) => c.id === incoming.id);
    if (existing && sameTarget(existing, incoming)) continue;
    let id = incoming.id;
    for (let n = 2; taken.has(id); n++) id = `${incoming.id}-${n}`;
    taken.add(id);
    if (id !== incoming.id) remap.set(incoming.id, id);
    customTargets.push({ ...incoming, id });
  }

  const allowedTargets = new Set([...knownTargetIds, ...customTargets.map((c) => c.id)]);
  const targets = parsed.targets.map((id) => remap.get(id) ?? id).filter((id) => allowedTargets.has(id));
  const tweaks = parsed.tweaks.filter((id) => knownTweakIds.includes(id));
  const dropped = parsed.targets.length - targets.length + (parsed.tweaks.length - tweaks.length);

  let n = existingModes.length + 1;
  while (existingModes.some((m) => m.id === `mode-${n}`)) n += 1;
  const names = new Set(existingModes.map((m) => m.name.toLowerCase()));
  let name = parsed.name;
  for (let k = 2; names.has(name.toLowerCase()); k++) name = `${parsed.name} (${k})`;

  const wantsPlan = GUID.test(parsed.powerPlan);
  const powerPlan = wantsPlan && installedPlanIds.includes(parsed.powerPlan) ? parsed.powerPlan : "";

  return {
    mode: { id: `mode-${n}`, name, targets: [...new Set(targets)], tweaks: [...new Set(tweaks)], powerPlan },
    customTargets,
    dropped,
    powerPlanDropped: wantsPlan && powerPlan === "",
  };
}