import type { ActionTemplate } from "./actionCatalog";

type Tone = "violet" | "sky" | "mint" | "gold" | "rose";

const TONES: Record<Tone, { accentColor: string; bgColor: string }> = {
  violet: { accentColor: "#8B5CF6", bgColor: "#231B38" },
  sky: { accentColor: "#56B9F2", bgColor: "#132235" },
  mint: { accentColor: "#35C98B", bgColor: "#112822" },
  gold: { accentColor: "#F4B860", bgColor: "#2A2215" },
  rose: { accentColor: "#F16B79", bgColor: "#2A1521" },
};

/**
 * One keyboard shortcut preset. The action id is `hotkey.<slug>` and the key combo is the
 * parameter, so presets stay editable in the key settings and share a single backend handler.
 */
function shortcut(
  slug: string,
  title: string,
  subtitle: string,
  iconName: string,
  combo: string,
  tone: Tone,
  description: string
): ActionTemplate {
  return {
    id: `hotkey.${slug}`,
    title,
    subtitle,
    category: "Shortcuts",
    iconName,
    actionLabel: `Shortcut: ${title} (${combo})`,
    defaultParam: combo,
    description,
    ...TONES[tone],
  };
}

function plain(
  id: string,
  category: ActionTemplate["category"],
  title: string,
  subtitle: string,
  iconName: string,
  actionLabel: string,
  defaultParam: string,
  tone: Tone,
  description: string
): ActionTemplate {
  return { id, title, subtitle, category, iconName, actionLabel, defaultParam, description, ...TONES[tone] };
}

const OBS_AND_MEDIA: ActionTemplate[] = [
  plain("obs.toggle_virtualcam", "OBS", "Virtual Cam", "VCAM", "Video", "OBS: Toggle virtual camera", "", "sky", "Start or stop the OBS virtual camera"),
  plain("obs.toggle_replay", "OBS", "Replay Buffer", "BUFFER", "Film", "OBS: Toggle replay buffer", "", "violet", "Start or stop the OBS replay buffer, so Clip has something to save"),
  plain("obs.pause_record", "OBS", "Pause Rec", "PAUSE", "Pause", "OBS: Pause or resume recording", "", "gold", "Pause or resume the current OBS recording"),
  plain("obs.studio_transition", "OBS", "Transition", "CUT", "Zap", "OBS: Studio mode transition", "", "sky", "Send the preview scene live when OBS is in Studio Mode"),
  plain("obs.set_scene", "OBS", "Go to Scene", "SCENE", "Clapperboard", "OBS: Switch to a named scene", "Main Scene", "sky", "Jump straight to one OBS scene by name"),
  plain("obs.toggle_mute", "OBS", "Mute Source", "MUTE", "MicOff", "OBS: Mute or unmute a source", "Mic/Aux", "rose", "Toggle mute on one OBS audio source by name"),
  plain("media.now_playing", "Media", "Now Playing", "ART", "Music", "Media: Now playing (album art)", "", "mint", "Shows the album art and song of what is playing. Press to play or pause"),
  plain("sys.cpu", "System", "CPU", "LIVE", "Cpu", "System: CPU use (live)", "", "sky", "Shows how busy the processor is, updated every two seconds. Press to open Task Manager"),
  plain("sys.ram", "System", "RAM", "LIVE", "MemoryStick", "System: Memory use (live)", "", "violet", "Shows how much memory is in use, updated every two seconds. Press to open Task Manager"),
  plain("sys.clock", "System", "Clock", "TIME", "Clock", "System: Clock", "", "gold", "Shows the time. Press to open the Clock app"),
  plain("media.stop", "Media", "Stop", "STOP", "Pause", "Media: Stop", "", "mint", "Stop playback in the current media app"),
];

const SHORTCUTS: ActionTemplate[] = [
  // Editing
  shortcut("copy", "Copy", "COPY", "Copy", "Ctrl+C", "violet", "Copy the selection"),
  shortcut("paste", "Paste", "PASTE", "Clipboard", "Ctrl+V", "violet", "Paste from the clipboard"),
  shortcut("cut", "Cut", "CUT", "Scissors", "Ctrl+X", "violet", "Cut the selection"),
  shortcut("undo", "Undo", "UNDO", "Undo", "Ctrl+Z", "violet", "Undo the last change"),
  shortcut("redo", "Redo", "REDO", "Redo", "Ctrl+Y", "violet", "Redo the last undone change"),
  shortcut("save", "Save", "SAVE", "Save", "Ctrl+S", "violet", "Save the current document"),
  shortcut("select_all", "Select All", "ALL", "LayoutGrid", "Ctrl+A", "violet", "Select everything"),
  // Windows and desktops
  shortcut("task_view", "Task View", "VIEW", "Layers", "Win+Tab", "sky", "Open Task View"),
  shortcut("switch_window", "Switch Window", "ALT+TAB", "Layers", "Alt+Tab", "sky", "Switch to the previous window"),
  shortcut("snap_left", "Snap Left", "LEFT", "ArrowLeft", "Win+Left", "sky", "Snap the active window to the left half"),
  shortcut("snap_right", "Snap Right", "RIGHT", "ArrowRight", "Win+Right", "sky", "Snap the active window to the right half"),
  shortcut("maximize", "Maximize", "MAX", "ArrowUp", "Win+Up", "sky", "Maximize the active window"),
  shortcut("minimize", "Minimize", "MIN", "ArrowDown", "Win+Down", "sky", "Minimize the active window"),
  shortcut("close_window", "Close Window", "CLOSE", "X", "Alt+F4", "rose", "Close the active window"),
  shortcut("desktop_left", "Desktop Left", "DESK", "ArrowLeft", "Win+Ctrl+Left", "sky", "Move to the virtual desktop on the left"),
  shortcut("desktop_right", "Desktop Right", "DESK", "ArrowRight", "Win+Ctrl+Right", "sky", "Move to the virtual desktop on the right"),
  shortcut("new_desktop", "New Desktop", "NEW", "Plus", "Win+Ctrl+D", "sky", "Create a new virtual desktop"),
  // Windows tools
  shortcut("clipboard_history", "Clipboard", "HISTORY", "Clipboard", "Win+V", "mint", "Open Windows clipboard history"),
  shortcut("emoji", "Emoji", "EMOJI", "Smile", "Win+.", "mint", "Open the emoji picker"),
  shortcut("voice_typing", "Voice Typing", "DICTATE", "Mic", "Win+H", "mint", "Start dictating with Windows voice typing"),
  shortcut("settings", "Settings", "SETTINGS", "Settings", "Win+I", "mint", "Open Windows Settings"),
  shortcut("explorer", "File Explorer", "FILES", "Folder", "Win+E", "mint", "Open File Explorer"),
  shortcut("quick_settings", "Quick Settings", "QUICK", "Sliders", "Win+A", "mint", "Open the Windows quick settings panel"),
  shortcut("game_bar", "Game Bar", "BAR", "Gamepad2", "Win+G", "mint", "Open the Xbox Game Bar"),
  shortcut("record_clip", "Record Last 30s", "CLIP", "Disc", "Win+Alt+G", "mint", "Game Bar: save the last 30 seconds of gameplay"),
  shortcut("record_toggle", "Game Record", "REC", "Disc", "Win+Alt+R", "rose", "Game Bar: start or stop recording"),
  // Browser
  shortcut("refresh", "Refresh", "F5", "RefreshCw", "F5", "gold", "Reload the page"),
  shortcut("new_tab", "New Tab", "TAB", "Plus", "Ctrl+T", "gold", "Open a new browser tab"),
  shortcut("close_tab", "Close Tab", "CLOSE", "X", "Ctrl+W", "gold", "Close the current tab"),
  shortcut("reopen_tab", "Reopen Tab", "REOPEN", "Undo", "Ctrl+Shift+T", "gold", "Reopen the last closed tab"),
  shortcut("next_tab", "Next Tab", "NEXT", "ArrowRight", "Ctrl+Tab", "gold", "Switch to the next tab"),
  shortcut("prev_tab", "Previous Tab", "PREV", "ArrowLeft", "Ctrl+Shift+Tab", "gold", "Switch to the previous tab"),
  shortcut("address_bar", "Address Bar", "URL", "Globe", "Ctrl+L", "gold", "Focus the browser address bar"),
];

export const EXTRA_ACTIONS: ActionTemplate[] = [...OBS_AND_MEDIA, ...SHORTCUTS];
