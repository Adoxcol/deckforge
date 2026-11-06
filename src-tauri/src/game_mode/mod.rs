//! Game Mode: closes the apps and stops the services you picked from what is running on this PC,
//! optionally switches to a power plan installed here, and puts everything back afterwards.
//!
//! Safety rules, enforced in code and not only in the settings:
//! * anti-cheat (Riot Vanguard, FACEIT, EasyAntiCheat, BattlEye), GPU and audio
//!   drivers, Defender, Explorer, Process Lasso and DeckForge itself are never
//!   matched, even when a target would otherwise catch them;
//! * stopping services needs administrator rights, so it goes through one
//!   elevated helper that DeckForge starts with a single UAC prompt. The helper
//!   only touches service names from a baked whitelist (never the deny list),
//!   restarts them when Game Mode ends, and stays resident until DeckForge exits,
//!   so there is only ever one prompt per session;
//! * a UAC prompt is never shown while a game is full-screen. The secure desktop
//!   switch resets the display driver and froze the PC for several seconds, so an
//!   automatic start only stops services when the helper is already running;
//! * work is spread out: one app at a time, on a background-priority thread, and
//!   an automatic start waits a few seconds after the game appears;
//! * IDEs get time to save and are left open if they still show a prompt.

use crate::drivers::hal::now_ms;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};
use tauri::Emitter;
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Threading::{
    GetCurrentThread, GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, SetPriorityClass,
    SetThreadPriority, TerminateProcess, HIGH_PRIORITY_CLASS, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SET_INFORMATION, PROCESS_TERMINATE, THREAD_MODE_BACKGROUND_BEGIN, THREAD_MODE_BACKGROUND_END,
};
use windows::Win32::UI::Shell::{IsUserAnAdmin, SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW,
    GetWindowThreadProcessId, IsWindowVisible, PostMessageW, GWL_EXSTYLE, GW_OWNER, WM_CLOSE, WS_EX_TOOLWINDOW,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const STILL_ACTIVE: u32 = 259;
const MB: u64 = 1024 * 1024;

mod catalog;
mod config;
mod power;
mod processes;
mod scan;
mod services;
mod session;
#[cfg(test)]
mod tests;
mod watcher;

pub use catalog::*;
pub use config::*;
use power::*;
use processes::*;
pub use scan::*;
use services::*;
pub use session::*;
pub use watcher::*;