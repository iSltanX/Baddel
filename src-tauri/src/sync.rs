//! Making a settings change take effect everywhere at once.
//!
//! Settings live in Rust, so every screen and the menu read the same values. When
//! one of them changes something, this is what tells the rest.

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::settings::{AppState, Binding, Settings};
use crate::{settings, shortcuts, tray};

/// Applies the difference between two settings snapshots and tells every open
/// window about the new one. Returns the shortcuts macOS refused to bind.
pub fn apply(app: &AppHandle, previous: &Settings, next: &Settings) -> Vec<Binding> {
    if previous.launch_at_login != next.launch_at_login {
        let autostart = app.autolaunch();
        let _ = if next.launch_at_login { autostart.enable() } else { autostart.disable() };
    }

    let rebind = Binding::ALL.iter().any(|&b| previous.binding(b) != next.binding(b));
    let refused = if rebind { shortcuts::apply(app, next) } else { Vec::new() };

    tray::rebuild(app);
    let _ = app.emit("settings", next);
    refused
}

/// The pause toggle, shared by the menu item and the global shortcut.
pub fn toggle_pause(app: &AppHandle) {
    let previous = app.state::<AppState>().get();
    let next = settings::update(app, |s| s.paused = !s.paused);
    apply(app, &previous, &next);
}
