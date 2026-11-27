import type { EncoderConfig, KeyConfig, ProfilePage, TouchZoneConfig } from "../types/deckforge";

export type TemplateCategory =
  | "Streaming"
  | "Media"
  | "Meetings"
  | "Work"
  | "Creative"
  | "Dev"
  | "Gaming"
  | "Links"
  | "Deck";

export const TEMPLATE_CATEGORIES: Array<TemplateCategory | "All" | "Mine"> = [
  "All",
  "Streaming",
  "Media",
  "Meetings",
  "Work",
  "Creative",
  "Dev",
  "Gaming",
  "Links",
  "Deck",
  "Mine",
];

export interface PageTemplate {
  id: string;
  name: string;
  tagline: string;
  category: TemplateCategory | "Mine";
  /** Colour of the card accent and the touch-strip label. */
  accent: string;
  builtIn: boolean;
  page: ProfilePage;
}

type Tone = "rose" | "violet" | "sky" | "mint" | "gold" | "slate";

const TONES: Record<Tone, { accent: string; bg: string }> = {
  rose: { accent: "#F16B79", bg: "#2A1521" },
  violet: { accent: "#8B5CF6", bg: "#231B38" },
  sky: { accent: "#56B9F2", bg: "#132235" },
  mint: { accent: "#35C98B", bg: "#112822" },
  gold: { accent: "#F4B860", bg: "#2A2215" },
  slate: { accent: "#A6AFC4", bg: "#1A1E27" },
};

const CATEGORY_OF_ACTION = (actionId: string): string => {
  if (actionId.startsWith("obs.")) return "OBS";
  if (actionId.startsWith("media.") || actionId.startsWith("audio.")) return "Media";
  if (actionId.startsWith("sys.") || actionId.startsWith("brightness.")) return "System";
  if (actionId.startsWith("profile.")) return "Navigation";
  if (actionId.startsWith("macro.")) return "Macro";
  if (actionId.startsWith("hotkey.")) return "Shortcuts";
  if (actionId === "none") return "Custom";
  return "Custom";
};

const LABEL_OF_ACTION: Record<string, string> = {
  "obs.start_stream": "OBS: Toggle stream",
  "obs.toggle_record": "OBS: Toggle recording",
  "obs.next_scene": "OBS: Next scene",
  "obs.prev_scene": "OBS: Previous scene",
  "obs.save_replay": "OBS: Save replay",
  "media.play_pause": "Media: Play / Pause",
  "media.next": "Media: Next track",
  "media.prev": "Media: Previous track",
  "media.stop": "Media: Stop",
  "audio.volume_up": "Audio: Volume up",
  "audio.volume_down": "Audio: Volume down",
  "audio.volume_mute": "Audio: Mute speakers",
  "audio.mute_mic": "Audio: Mute microphone",
  "audio.open_mixer": "Audio: Open mixer",
  "sys.show_desktop": "System: Show desktop",
  "sys.screenshot": "System: Screenshot",
  "sys.task_manager": "System: Task Manager",
  "sys.lock_pc": "System: Lock PC",
  "sys.game_mode": "System: Game Mode",
  "brightness.up": "Deck: Brightness up",
  "brightness.down": "Deck: Brightness down",
  "brightness.toggle": "Deck: Backlight on / off",
  "profile.next_page": "Navigation: Next page",
  "profile.prev_page": "Navigation: Previous page",
  "profile.switch": "Navigation: Switch profile",
  "macro.run": "Macro: Run",
  "custom.hotkey": "Custom: Keyboard shortcut",
  "custom.url": "Custom: Open link",
};

/** One key: title, small caption, icon, action, action parameter, colour. */
type KeySpec = [string, string, string, string, string, Tone];

const key = (index: number, spec: KeySpec): KeyConfig => {
  const [title, subtitle, iconName, actionId, actionParam, tone] = spec;
  const colors = TONES[tone];
  return {
    index,
    title,
    subtitle,
    iconName,
    actionId,
    actionLabel: LABEL_OF_ACTION[actionId] ?? title,
    actionCategory: CATEGORY_OF_ACTION(actionId),
    actionParam,
    accentColor: colors.accent,
    bgColor: colors.bg,
    showTitle: true,
    titleSize: "Medium",
    backgroundStyle: "Gradient",
    cornerRadius: 8,
  };
};

const emptyKey = (index: number): KeyConfig => ({
  index,
  title: "Empty",
  subtitle: "",
  iconName: "Plus",
  actionId: "none",
  actionLabel: "Unassigned",
  actionCategory: "Custom",
  actionParam: "",
  accentColor: "#707B94",
  bgColor: "#151925",
  showTitle: true,
  titleSize: "Medium",
  backgroundStyle: "Gradient",
  cornerRadius: 8,
});

const KEY_SLOTS = 32;

const hotkey = (title: string, subtitle: string, icon: string, chord: string, tone: Tone): KeySpec => [
  title,
  subtitle,
  icon,
  "custom.hotkey",
  chord,
  tone,
];

const link = (title: string, icon: string, url: string, tone: Tone): KeySpec => [
  title,
  "OPEN",
  icon,
  "custom.url",
  url,
  tone,
];

const zones = (first: TouchZoneConfig): TouchZoneConfig[] => [
  first,
  { index: 1, label: "PREV PAGE", valueText: "Previous page", tapActionId: "profile.prev_page", accentColor: "#56B9F2" },
  { index: 2, label: "PAGE", valueText: "Swipe to change", tapActionId: "profile.next_page", accentColor: "#8B5CF6" },
  { index: 3, label: "NEXT PAGE", valueText: "Next page", tapActionId: "profile.next_page", accentColor: "#F4B860" },
];

const STANDARD_ENCODERS: EncoderConfig[] = [
  { index: 0, label: "Master Vol", valueText: "Win32 Vol ±", cwAction: "audio.volume_up", ccwAction: "audio.volume_down", pushAction: "audio.volume_mute" },
  { index: 1, label: "Media Track", valueText: "Prev / Next", cwAction: "media.next", ccwAction: "media.prev", pushAction: "media.play_pause" },
  { index: 2, label: "Timeline", valueText: "Left / Right", cwAction: "Right", ccwAction: "Left", pushAction: "Space" },
  { index: 3, label: "Brightness", valueText: "Backlight", cwAction: "brightness.up", ccwAction: "brightness.down", pushAction: "brightness.toggle" },
];

const buildPage = (
  id: string,
  name: string,
  specs: KeySpec[],
  firstZone: TouchZoneConfig,
): ProfilePage => ({
  id,
  name,
  keys: Array.from({ length: KEY_SLOTS }, (_, i) => (specs[i] ? key(i, specs[i]) : emptyKey(i))),
  touchZones: zones(firstZone),
  encoders: JSON.parse(JSON.stringify(STANDARD_ENCODERS)),
});

interface TemplateDef {
  id: string;
  name: string;
  tagline: string;
  category: TemplateCategory;
  accent: string;
  specs: KeySpec[];
  zone: TouchZoneConfig;
}

const DEFS: TemplateDef[] = [
  {
    id: "tpl-nvidia",
    name: "NVIDIA Capture",
    tagline: "Instant Replay, clips, recording and the overlay from the NVIDIA app. Uses its default shortcuts.",
    category: "Gaming",
    accent: "#76B900",
    specs: [
      hotkey("Save Clip", "ALT+F10", "Scissors", "Alt+F10", "mint"),
      hotkey("Replay", "ALT+SHIFT+F10", "RefreshCw", "Alt+Shift+F10", "mint"),
      hotkey("Record", "ALT+F9", "CircleDot", "Alt+F9", "rose"),
      hotkey("Screenshot", "ALT+F1", "Camera", "Alt+F1", "sky"),
      hotkey("Overlay", "ALT+Z", "Layers", "Alt+Z", "mint"),
      hotkey("Stats", "ALT+R", "Cpu", "Alt+R", "gold"),
      hotkey("Photo Mode", "ALT+F2", "Sparkles", "Alt+F2", "violet"),
      hotkey("Filters", "ALT+F3", "Sliders", "Alt+F3", "violet"),
      ["Mic", "MUTE", "MicOff", "audio.mute_mic", "", "rose"],
      ["Game Mode", "FOCUS", "Gamepad2", "sys.game_mode", "", "slate"],
    ],
    zone: { index: 0, label: "SAVE CLIP", valueText: "Last few minutes", tapActionId: "custom.hotkey", tapActionParam: "Alt+F10", accentColor: "#76B900" },
  },
  {
    id: "tpl-go-live",
    name: "Go Live",
    tagline: "Stream, record, switch scenes, clip the last minute.",
    category: "Streaming",
    accent: "#F16B79",
    specs: [
      ["Go Live", "LIVE", "Radio", "obs.start_stream", "", "rose"],
      ["Record", "REC", "CircleDot", "obs.toggle_record", "", "rose"],
      ["Scene -", "PREV", "Film", "obs.prev_scene", "", "sky"],
      ["Scene +", "NEXT", "Clapperboard", "obs.next_scene", "", "sky"],
      ["Clip", "REPLAY", "Scissors", "obs.save_replay", "", "violet"],
      ["Mic", "MUTE", "MicOff", "audio.mute_mic", "", "rose"],
      ["Mute", "SOUND", "Volume2", "audio.volume_mute", "", "gold"],
      ["Vol -", "-2%", "Volume2", "audio.volume_down", "", "mint"],
      ["Vol +", "+2%", "Volume2", "audio.volume_up", "", "mint"],
      ["Snap", "SHOT", "Camera", "sys.screenshot", "", "sky"],
    ],
    zone: { index: 0, label: "GO LIVE", valueText: "Start / stop stream", tapActionId: "obs.start_stream", accentColor: "#F16B79" },
  },
  {
    id: "tpl-music",
    name: "Music Deck",
    tagline: "Transport, volume and the mixer for Spotify or any player.",
    category: "Media",
    accent: "#35C98B",
    specs: [
      ["Play", "PAUSE", "Play", "media.play_pause", "", "mint"],
      ["Next", "TRACK", "Music", "media.next", "", "mint"],
      ["Back", "TRACK", "Music", "media.prev", "", "mint"],
      ["Stop", "MEDIA", "Pause", "media.stop", "", "slate"],
      ["Mixer", "AUDIO", "Sliders", "audio.open_mixer", "", "violet"],
      ["Vol -", "-2%", "Volume2", "audio.volume_down", "", "sky"],
      ["Vol +", "+2%", "Volume2", "audio.volume_up", "", "sky"],
      ["Mute", "SOUND", "Volume2", "audio.volume_mute", "", "gold"],
      ["Mic", "MUTE", "MicOff", "audio.mute_mic", "", "rose"],
      ["Desktop", "SHOW", "Monitor", "sys.show_desktop", "", "slate"],
    ],
    zone: { index: 0, label: "PLAY", valueText: "Play / Pause", tapActionId: "media.play_pause", accentColor: "#35C98B" },
  },
  {
    id: "tpl-meeting",
    name: "Meeting Room",
    tagline: "Mute in Teams, Zoom or Meet with one press, without hunting for the window.",
    category: "Meetings",
    accent: "#56B9F2",
    specs: [
      ["Mic", "MUTE", "MicOff", "audio.mute_mic", "", "rose"],
      hotkey("Teams", "MUTE", "MessageSquare", "Ctrl+Shift+M", "violet"),
      hotkey("Zoom", "MUTE", "Mic", "Alt+A", "sky"),
      hotkey("Zoom", "VIDEO", "Video", "Alt+V", "sky"),
      hotkey("Meet", "MUTE", "Mic", "Ctrl+D", "mint"),
      ["Mute", "SOUND", "Volume2", "audio.volume_mute", "", "gold"],
      ["Vol -", "-2%", "Volume2", "audio.volume_down", "", "mint"],
      ["Vol +", "+2%", "Volume2", "audio.volume_up", "", "mint"],
      ["Snap", "SHOT", "Camera", "sys.screenshot", "", "sky"],
      ["Lock", "PC", "Shield", "sys.lock_pc", "", "slate"],
    ],
    zone: { index: 0, label: "MIC", valueText: "Mute / unmute", tapActionId: "audio.mute_mic", accentColor: "#F16B79" },
  },
  {
    id: "tpl-office",
    name: "Office Essentials",
    tagline: "Files, clipboard, task manager and the shortcuts you reach for all day.",
    category: "Work",
    accent: "#F4B860",
    specs: [
      hotkey("Files", "WIN+E", "Folder", "Win+E", "gold"),
      ["Snap", "SHOT", "Camera", "sys.screenshot", "", "sky"],
      hotkey("Tasks", "MGR", "Terminal", "Ctrl+Shift+Esc", "violet"),
      ["Desktop", "SHOW", "Monitor", "sys.show_desktop", "", "slate"],
      ["Lock", "PC", "Shield", "sys.lock_pc", "", "slate"],
      hotkey("Copy", "CTRL+C", "Layers", "Ctrl+C", "gold"),
      hotkey("Paste", "CTRL+V", "Layers", "Ctrl+V", "gold"),
      hotkey("Undo", "CTRL+Z", "Sliders", "Ctrl+Z", "sky"),
      hotkey("Redo", "CTRL+Y", "Sliders", "Ctrl+Y", "sky"),
      hotkey("Clips", "WIN+V", "LayoutGrid", "Win+V", "mint"),
    ],
    zone: { index: 0, label: "FILES", valueText: "Open Explorer", tapActionId: "custom.hotkey", accentColor: "#F4B860", tapActionParam: "Win+E" },
  },
  {
    id: "tpl-edit",
    name: "Edit Suite",
    tagline: "Undo, redo, cut, copy, paste, zoom and save for any editor.",
    category: "Creative",
    accent: "#8B5CF6",
    specs: [
      hotkey("Undo", "CTRL+Z", "Sliders", "Ctrl+Z", "violet"),
      hotkey("Redo", "CTRL+Y", "Sliders", "Ctrl+Y", "violet"),
      hotkey("Save", "CTRL+S", "Zap", "Ctrl+S", "mint"),
      hotkey("Cut", "CTRL+X", "Scissors", "Ctrl+X", "rose"),
      hotkey("Copy", "CTRL+C", "Layers", "Ctrl+C", "sky"),
      hotkey("Paste", "CTRL+V", "Layers", "Ctrl+V", "sky"),
      hotkey("All", "CTRL+A", "LayoutGrid", "Ctrl+A", "slate"),
      hotkey("Zoom +", "CTRL +", "Plus", "Ctrl+Plus", "gold"),
      hotkey("Zoom -", "CTRL -", "Sliders", "Ctrl+Minus", "gold"),
      ["Snap", "SHOT", "Camera", "sys.screenshot", "", "sky"],
    ],
    zone: { index: 0, label: "SAVE", valueText: "Ctrl + S", tapActionId: "custom.hotkey", accentColor: "#35C98B", tapActionParam: "Ctrl+S" },
  },
  {
    id: "tpl-dev",
    name: "Dev Console",
    tagline: "VS Code palette, search, run and stop, plus your dev workspace macro.",
    category: "Dev",
    accent: "#35C98B",
    specs: [
      hotkey("Palette", "CTRL+SHIFT+P", "Terminal", "Ctrl+Shift+P", "violet"),
      hotkey("Open", "CTRL+P", "Folder", "Ctrl+P", "sky"),
      hotkey("Find", "ALL FILES", "Cpu", "Ctrl+Shift+F", "sky"),
      hotkey("Sidebar", "CTRL+B", "LayoutGrid", "Ctrl+B", "slate"),
      hotkey("Panel", "CTRL+J", "Terminal", "Ctrl+J", "slate"),
      hotkey("Run", "F5", "Play", "F5", "mint"),
      hotkey("Stop", "SHIFT+F5", "Pause", "Shift+F5", "rose"),
      hotkey("Save", "CTRL+S", "Zap", "Ctrl+S", "gold"),
      ["Workspace", "MACRO", "Sparkles", "macro.run", "macro-dev-workspace", "violet"],
      hotkey("Tasks", "MGR", "Cpu", "Ctrl+Shift+Esc", "slate"),
    ],
    zone: { index: 0, label: "WORKSPACE", valueText: "Run dev macro", tapActionId: "macro.run", accentColor: "#8B5CF6", tapActionParam: "macro-dev-workspace" },
  },
  {
    id: "tpl-game",
    name: "Game Mode",
    tagline: "Game Bar, clips, screenshots and a one-press mic mute.",
    category: "Gaming",
    accent: "#35C98B",
    specs: [
      hotkey("Game Bar", "WIN+G", "Gamepad2", "Win+G", "mint"),
      hotkey("Clip", "WIN+ALT+R", "CircleDot", "Win+Alt+R", "rose"),
      hotkey("Shot", "WIN+ALT+PRTSC", "Camera", "Win+Alt+PrtSc", "sky"),
      ["Mic", "MUTE", "MicOff", "audio.mute_mic", "", "rose"],
      hotkey("Discord", "MUTE", "MessageSquare", "Ctrl+Shift+M", "violet"),
      ["Vol -", "-2%", "Volume2", "audio.volume_down", "", "mint"],
      ["Vol +", "+2%", "Volume2", "audio.volume_up", "", "mint"],
      ["Mute", "SOUND", "Volume2", "audio.volume_mute", "", "gold"],
      hotkey("Tasks", "MGR", "Cpu", "Ctrl+Shift+Esc", "slate"),
      ["Game Mode", "BOOST", "Zap", "sys.game_mode", "", "gold"],
    ],
    zone: { index: 0, label: "MIC", valueText: "Mute / unmute", tapActionId: "audio.mute_mic", accentColor: "#35C98B" },
  },
  {
    id: "tpl-links",
    name: "Quick Links",
    tagline: "Ten sites one press away. Change any URL in the inspector.",
    category: "Links",
    accent: "#56B9F2",
    specs: [
      link("YouTube", "Video", "https://www.youtube.com", "rose"),
      link("GitHub", "Terminal", "https://github.com", "slate"),
      link("Gmail", "MessageSquare", "https://mail.google.com", "rose"),
      link("Calendar", "Briefcase", "https://calendar.google.com", "sky"),
      link("Drive", "Folder", "https://drive.google.com", "gold"),
      link("Docs", "Layers", "https://docs.google.com", "sky"),
      link("Twitch", "Radio", "https://www.twitch.tv", "violet"),
      link("Maps", "Globe", "https://maps.google.com", "mint"),
      link("News", "Globe", "https://news.google.com", "slate"),
      link("Search", "Globe", "https://www.google.com", "gold"),
    ],
    zone: { index: 0, label: "WEB", valueText: "Open Google", tapActionId: "custom.url", accentColor: "#56B9F2", tapActionParam: "https://www.google.com" },
  },
  {
    id: "tpl-deck",
    name: "Deck Control",
    tagline: "Switch profiles, change pages, dim the backlight, run your go-live macro.",
    category: "Deck",
    accent: "#A6AFC4",
    specs: [
      ["Streaming", "PROFILE", "Radio", "profile.switch", "profile-streaming", "rose"],
      ["Gaming", "PROFILE", "Gamepad2", "profile.switch", "profile-gaming", "mint"],
      ["Editing", "PROFILE", "Film", "profile.switch", "profile-editing", "sky"],
      ["Work", "PROFILE", "Briefcase", "profile.switch", "profile-work", "gold"],
      ["Go Live", "MACRO", "Sparkles", "macro.run", "macro-go-live", "violet"],
      ["Page -", "PREV", "Layers", "profile.prev_page", "", "slate"],
      ["Page +", "NEXT", "Layers", "profile.next_page", "", "slate"],
      ["Dim", "DECK", "Zap", "brightness.down", "", "slate"],
      ["Bright", "DECK", "Zap", "brightness.up", "", "gold"],
      ["Backlight", "ON / OFF", "Lightbulb", "brightness.toggle", "", "violet"],
    ],
    zone: { index: 0, label: "PROFILE", valueText: "Go to Streaming", tapActionId: "profile.switch", accentColor: "#F16B79", tapActionParam: "profile-streaming" },
  },
  {
    id: "tpl-blank",
    name: "Blank Page",
    tagline: "Every key empty. Only the touch strip page buttons are set.",
    category: "Deck",
    accent: "#707B94",
    specs: [],
    zone: { index: 0, label: "HOME", valueText: "Tap to assign", tapActionId: "none", accentColor: "#707B94" },
  },
];

export const BUILT_IN_TEMPLATES: PageTemplate[] = DEFS.map((def) => ({
  id: def.id,
  name: def.name,
  tagline: def.tagline,
  category: def.category,
  accent: def.accent,
  builtIn: true,
  page: buildPage(`page-${def.id}`, def.name, def.specs, def.zone),
}));

/** A fresh, independent page built from a template. */
export const instantiateTemplate = (template: PageTemplate, name: string, id?: string): ProfilePage => {
  const copy: ProfilePage = JSON.parse(JSON.stringify(template.page));
  copy.id = id ?? `page-${template.id}-${Date.now()}`;
  copy.name = name.trim() || template.name;
  return copy;
};

const STORAGE_KEY = "deckforge.pageTemplates.v1";

export const loadCustomTemplates = (): PageTemplate[] => {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as PageTemplate[];
    return Array.isArray(parsed)
      ? parsed.filter((tpl) => tpl && tpl.page && Array.isArray(tpl.page.keys)).map((tpl) => ({ ...tpl, builtIn: false, category: "Mine" }))
      : [];
  } catch {
    return [];
  }
};

export const saveCustomTemplates = (templates: PageTemplate[]): void => {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(templates));
  } catch {
    // Storage can be unavailable in private windows. The gallery still works for this session.
  }
};

export const templateFromPage = (page: ProfilePage, name: string): PageTemplate => {
  const firstKey = page.keys.find((k) => k.actionId !== "none");
  return {
    id: `tpl-mine-${Date.now()}`,
    name: name.trim() || page.name,
    tagline: `Saved from "${page.name}".`,
    category: "Mine",
    accent: firstKey?.accentColor ?? "#8B5CF6",
    builtIn: false,
    page: JSON.parse(JSON.stringify(page)),
  };
};
