//! User preferences: the single source of truth for everything the UI can change.
//!
//! Held in memory behind a mutex and mirrored to `settings.json` through the store
//! plugin. Only preferences live here — never text the user typed.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;

/// Apps where the hotkey does nothing: terminals (the selection keys mean something
/// else there) and password managers. Each entry records the defaults version that
/// introduced it, so an existing install picks up later additions exactly once.
const DEFAULT_EXCLUDED: [(&str, u32); 14] = [
    // Terminals
    ("com.apple.Terminal", 1),
    ("com.googlecode.iterm2", 1),
    ("dev.warp.Warp-Stable", 2),
    ("com.mitchellh.ghostty", 2),
    ("net.kovidgoyal.kitty", 2),
    ("org.alacritty", 2),
    ("com.github.wez.wezterm", 2),
    // Password managers
    ("com.apple.keychainaccess", 1),
    ("com.apple.Passwords", 2),
    ("com.1password.1password", 1),
    ("com.agilebits.onepassword7", 1),
    ("com.bitwarden.desktop", 2),
    ("org.keepassxc.keepassxc", 2),
    ("in.sinew.Enpass-Desktop", 2),
];

/// Bump when entries are added to [`DEFAULT_EXCLUDED`].
const DEFAULTS_VERSION: u32 = 2;

pub fn is_default_excluded(bundle_id: &str) -> bool {
    DEFAULT_EXCLUDED.iter().any(|(id, _)| *id == bundle_id)
}

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
    /// When updates were last checked, in milliseconds since the Unix epoch.
    pub last_update_check: Option<u64>,
    /// The [`DEFAULTS_VERSION`] these settings have seen. Files written before it
    /// existed had the first set of defaults, hence 1 there, not the current version.
    #[serde(default = "first_defaults")]
    pub defaults_version: u32,
}

fn first_defaults() -> u32 {
    1
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
            excluded_apps: DEFAULT_EXCLUDED.iter().map(|(id, _)| id.to_string()).collect(),
            paused: false,
            welcomed: false,
            last_update_check: None,
            defaults_version: DEFAULTS_VERSION,
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
    let Some(mut settings) = store.get(STORE_KEY).and_then(|value| serde_json::from_value::<Settings>(value).ok())
    else {
        return Settings::default();
    };
    if settings.adopt_new_defaults() {
        save(app, &settings);
    }
    settings
}

impl Settings {
    /// Adds the default exceptions introduced since these settings were written. One the
    /// user removed after it arrived is not brought back: each is offered only once.
    fn adopt_new_defaults(&mut self) -> bool {
        if self.defaults_version >= DEFAULTS_VERSION {
            return false;
        }
        for (id, since) in DEFAULT_EXCLUDED {
            if since > self.defaults_version && !self.excluded_apps.iter().any(|e| e == id) {
                self.excluded_apps.push(id.to_string());
            }
        }
        self.defaults_version = DEFAULTS_VERSION;
        true
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_from_before_the_defaults_version_gets_the_new_exceptions_once() {
        let json = serde_json::json!({ "excludedApps": ["com.apple.Terminal", "com.example.mine"] });
        let mut settings: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(settings.defaults_version, 1);
        assert!(settings.adopt_new_defaults());
        assert!(settings.excluded_apps.contains(&"com.apple.Passwords".to_string()));
        assert!(settings.excluded_apps.contains(&"com.example.mine".to_string()));
        // Entries from version 1 that the user had removed stay removed.
        assert!(!settings.excluded_apps.contains(&"com.googlecode.iterm2".to_string()));

        settings.excluded_apps.retain(|id| id != "com.apple.Passwords");
        assert!(!settings.adopt_new_defaults());
        assert!(!settings.excluded_apps.contains(&"com.apple.Passwords".to_string()));
    }

    #[test]
    fn fresh_settings_are_current() {
        let mut settings = Settings::default();
        assert!(!settings.adopt_new_defaults());
        assert_eq!(settings.excluded_apps.len(), DEFAULT_EXCLUDED.len());
    }
}
