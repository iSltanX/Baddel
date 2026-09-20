use std::sync::Mutex;

/// Apps where the hotkey does nothing: terminals (word selection keys mean something
/// else there) and password managers.
const DEFAULT_EXCLUDED: [&str; 5] = [
    "com.apple.Terminal",
    "com.googlecode.iterm2",
    "com.apple.keychainaccess",
    "com.1password.1password",
    "com.agilebits.onepassword7",
];

/// User preferences. In memory for now; persisted through the store plugin in phase 3.
#[derive(Clone, Debug)]
pub struct Settings {
    pub switch_input_source: bool,
    pub paused: bool,
    pub excluded_apps: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            switch_input_source: true,
            paused: false,
            excluded_apps: DEFAULT_EXCLUDED.map(String::from).to_vec(),
        }
    }
}

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
}
