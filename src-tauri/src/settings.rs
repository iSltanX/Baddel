//! User preferences: the single source of truth for everything the UI can change.
//!
//! Held in memory behind a mutex and mirrored to `settings.json` through the store
//! plugin. Only preferences live here — never text the user typed.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;

/// Apps where the hotkey does nothing: terminals (the selection keys mean something
/// else there) and password managers.
const DEFAULT_EXCLUDED: [&str; 5] = [
    "com.apple.Terminal",
    "com.googlecode.iterm2",
    "com.apple.keychainaccess",
    "com.1password.1password",
    "com.agilebits.onepassword7",
];

pub const DEFAULT_SHORTCUT: &str = "Alt+Shift+Space";

const STORE_FILE: &str = "settings.json";
const STORE_KEY: &str = "settings";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Ar,
    En,
}

/// Which of the three shortcuts a binding belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Binding {
    Convert,
    Undo,
    Pause,
}

impl Binding {
    pub const ALL: [Binding; 3] = [Binding::Convert, Binding::Undo, Binding::Pause];
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    // عام
    pub launch_at_login: bool,
    pub show_tray_icon: bool,
    pub language: Language,
    pub switch_input_source: bool,
    pub show_hud: bool,
    pub sound: bool,
    pub auto_update: bool,
    // الاختصارات — an empty string means "not bound".
    pub shortcut_convert: String,
    pub shortcut_undo: String,
    pub shortcut_pause: String,
    // التخطيطات — an input source id, or empty to follow whatever macOS has enabled.
    pub arabic_layout: String,
    pub latin_layout: String,
    // الاستثناءات — bundle identifiers. Names and icons are resolved live.
    pub excluded_apps: Vec<String>,
    // Not shown in the UI.
    pub paused: bool,
    /// Set once the welcome window has been completed or skipped.
    pub welcomed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            launch_at_login: false,
            show_tray_icon: true,
            language: Language::Ar,
            switch_input_source: true,
            show_hud: true,
            sound: false,
            auto_update: true,
            shortcut_convert: DEFAULT_SHORTCUT.to_string(),
            shortcut_undo: String::new(),
            shortcut_pause: String::new(),
            arabic_layout: String::new(),
            latin_layout: String::new(),
            excluded_apps: DEFAULT_EXCLUDED.map(String::from).to_vec(),
            paused: false,
            welcomed: false,
        }
    }
}

impl Settings {
    pub fn binding(&self, which: Binding) -> &str {
        match which {
            Binding::Convert => &self.shortcut_convert,
            Binding::Undo => &self.shortcut_undo,
            Binding::Pause => &self.shortcut_pause,
        }
    }

    pub fn set_binding(&mut self, which: Binding, accelerator: String) {
        match which {
            Binding::Convert => self.shortcut_convert = accelerator,
            Binding::Undo => self.shortcut_undo = accelerator,
            Binding::Pause => self.shortcut_pause = accelerator,
        }
    }
}

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
}

impl AppState {
    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
}

/// Reads the stored preferences, falling back to the defaults for anything missing
/// or unreadable. A corrupt store must never stop the app from starting.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Settings {
    let Ok(store) = app.store(STORE_FILE) else { return Settings::default() };
    store
        .get(STORE_KEY)
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

/// Writes the preferences out. Failures are not fatal: the in-memory settings stay
/// authoritative for this run.
pub fn save<R: Runtime>(app: &AppHandle<R>, settings: &Settings) {
    let Ok(store) = app.store(STORE_FILE) else { return };
    let Ok(value) = serde_json::to_value(settings) else { return };
    store.set(STORE_KEY, value);
    let _ = store.save();
}

/// Applies `change` to the settings, persists the result, and hands it back.
pub fn update<R, F>(app: &AppHandle<R>, change: F) -> Settings
where
    R: Runtime,
    F: FnOnce(&mut Settings),
{
    let state = app.state::<AppState>();
    let updated = {
        let mut settings = state.settings.lock().unwrap();
        change(&mut settings);
        settings.clone()
    };
    save(app, &updated);
    updated
}
