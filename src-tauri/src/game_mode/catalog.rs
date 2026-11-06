// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------
//
// Game Mode ships with no opinion about what to close. The only built-in presets are a few
// launchers that need special handling (several processes, or a VM to shut down). Everything
// else comes from the programs and services the user picks from their own PC.

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Reopen {
    /// Start the recorded exe (with its original arguments) again.
    Exe,
    /// Nothing to reopen (services come back on their own, dev servers do not).
    No,
}

pub(super) struct TargetDef {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    /// "apps" or "background".
    pub(super) group: &'static str,
    pub(super) note: &'static str,
    /// Lowercase exe names without ".exe". The first entry is the main program.
    pub(super) procs: &'static [&'static str],
    /// Lowercase name prefixes.
    pub(super) prefixes: &'static [&'static str],
    /// Windows services stopped (and restarted afterwards) through the helper.
    pub(super) services: &'static [&'static str],
    /// Ask windows to close first, so apps can save and exit cleanly.
    pub(super) graceful: bool,
    /// Kill whatever is still running after the grace period. Off for editors.
    pub(super) force: bool,
    pub(super) reopen: Reopen,
    /// Run `wsl --shutdown` once the processes are gone.
    pub(super) wsl_shutdown: bool,
    /// Keep DeckForge's own dev stack (vite, tauri) alive.
    pub(super) protect_dev: bool,
}

pub(super) const BASE: TargetDef = TargetDef {
    id: "",
    label: "",
    group: "apps",
    note: "",
    procs: &[],
    prefixes: &[],
    services: &[],
    graceful: false,
    force: true,
    reopen: Reopen::No,
    wsl_shutdown: false,
    protect_dev: false,
};

pub(super) const TARGETS: &[TargetDef] = &[
    TargetDef {
        id: "discord",
        label: "Discord",
        note: "You leave voice chat.",
        procs: &["discord"],
        graceful: true,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "spotify",
        label: "Spotify",
        note: "Music stops.",
        procs: &["spotify"],
        graceful: true,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "steam",
        label: "Steam client",
        note: "Keep it open if the game starts from Steam.",
        procs: &["steam", "steamwebhelper"],
        graceful: true,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "epic",
        label: "Epic Games Launcher",
        note: "Keep it open if the game starts from Epic.",
        procs: &["epicgameslauncher", "epicwebhelper"],
        graceful: true,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "riotclient",
        label: "Riot Client launcher",
        note: "Closes the launcher only. Vanguard anti-cheat is never touched.",
        procs: &["riotclientservices", "riotclientux", "riotclientuxrender"],
        graceful: true,
        reopen: Reopen::Exe,
        ..BASE
    },
    TargetDef {
        id: "docker_wsl",
        label: "Docker Desktop + WSL",
        note: "Stops Docker and shuts the WSL VM down, which frees about 1.5 GB. Running containers stop.",
        procs: &["docker desktop"],
        prefixes: &["com.docker."],
        reopen: Reopen::Exe,
        wsl_shutdown: true,
        ..BASE
    },
];

/// Services paused by the "pause_updates" tweak.
pub(super) const UPDATE_SERVICES: &[&str] = &["wuauserv", "UsoSvc", "DoSvc", "BITS"];

pub(super) struct TweakDef {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) note: &'static str,
}

pub(super) const TWEAKS: &[TweakDef] = &[
    TweakDef {
        id: "priority",
        label: "Raise the game's priority to High",
        note: "Applies to the full-screen game as soon as DeckForge sees it.",
    },
    TweakDef {
        id: "pause_updates",
        label: "Pause Windows Update and Delivery Optimization",
        note: "Stops the update services while you play. Needs the admin helper.",
    },
];

/// Safety net: names that no target may ever touch.
pub(super) const NEVER_KILL_NAMES: &[&str] = &[
    "explorer",
    "dwm",
    "csrss",
    "winlogon",
    "lsass",
    "services",
    "svchost",
    "system",
    "smss",
    "wininit",
    "audiodg",
    "deckforge",
    "processlasso",
    "processgovernor",
    "voicemeeter8x64",
    "vgc",
    "vgtray",
    "msmpeng",
    "nvcontainer",
    "nvdisplay.container",
];

pub(super) const NEVER_KILL_PATHS: &[&str] = &[
    "\\riot vanguard\\",
    "\\easyanticheat",
    "\\battleye",
    "\\faceit anti",
    "\\nvidia corporation\\",
    "\\windows defender\\",
    "\\windows\\system32\\",
];

/// Shown in the UI so it is clear what Game Mode leaves alone.
pub const PROTECTED_NOTES: &[&str] = &[
    "Anti-cheat such as Riot Vanguard, EasyAntiCheat, BattlEye and FACEIT",
    "Graphics driver and overlay services",
    "Windows audio and audio routing software",
    "Windows Defender and security services",
    "Explorer, DWM and core Windows processes",
    "Process managers such as Process Lasso",
    "DeckForge itself",
];

/// Programs that are never treated as a game, even when full-screen.
pub(super) const NOT_GAMES: &[&str] = &[
    "explorer", "searchhost", "shellexperiencehost", "applicationframehost", "lockapp",
    "chrome", "msedge", "firefox", "brave", "opera", "vivaldi", "zen", "vlc", "mpc-hc64", "potplayermini64",
    "cursor", "code", "spotify", "discord", "obs64", "deckforge", "teams", "ms-teams",
    "zoom", "slack", "mstsc", "vmconnect",
];

/// Install folders that mark a full-screen window as a game.
pub(super) const GAME_PATHS: &[&str] = &[
    "\\steamapps\\common\\",
    "\\riot games\\",
    "\\epic games\\",
    "\\faceit",
    "\\battle.net",
    "\\ubisoft",
    "\\ea games\\",
    "\\gog galaxy\\games\\",
    "\\xboxgames\\",
    "\\roblox\\",
    "\\rockstar games\\",
    "\\origin games\\",
];
