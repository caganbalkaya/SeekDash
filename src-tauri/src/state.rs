use crate::{
    clipboard::ClipboardStore,
    database::Database,
    models::{AppSettings, IndexStatus},
};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, AtomicU64},
    time::Instant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowPhase {
    Hidden,
    Opening,
    Visible,
    Closing,
    SettingsVisible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarState {
    Hidden,
    Collapsed,
    Expanded,
}

impl SidebarState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Collapsed => "collapsed",
            Self::Expanded => "expanded",
        }
    }
}

pub struct WindowRuntime {
    pub phase: WindowPhase,
    pub sidebar_state: SidebarState,
    pub settings_ready: bool,
    pub settings_create_started: Option<Instant>,
    pub settings_open_started: Option<Instant>,
    pub clipboard_visible: bool,
    pub launcher_shown_at: Option<Instant>,
}

impl Default for WindowRuntime {
    fn default() -> Self {
        Self {
            phase: WindowPhase::Hidden,
            sidebar_state: SidebarState::Hidden,
            settings_ready: false,
            settings_create_started: None,
            settings_open_started: None,
            clipboard_visible: false,
            launcher_shown_at: None,
        }
    }
}

pub struct EngineState {
    pub db: Mutex<Database>,
    pub search_db: Mutex<Database>,
    pub settings: Mutex<AppSettings>,
    pub status: Mutex<IndexStatus>,
    pub latest_request: AtomicU64,
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
    pub icon_cache: Mutex<HashMap<String, Option<String>>>,
    pub shortcut_cache: Mutex<HashMap<String, String>>,
    pub clipboard: ClipboardStore,
    pub hotkey_error: Mutex<Option<String>>,
    pub settings_save: Mutex<()>,
    pub hotkey_registration: Mutex<()>,
    pub hotkey_recovery_running: AtomicBool,
    pub windows: Mutex<WindowRuntime>,
}
