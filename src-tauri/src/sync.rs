//! Making a settings change take effect everywhere at once.
//!
//! Settings live in Rust, so every screen and the menu read the same values. When
//! one of them changes something, this is what tells the rest.

use std::sync::atomic::{AtomicU64, Ordering};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::settings::{AppState, Binding, Settings};
use crate::sys::clock;
use crate::{settings, shortcuts, tray};

/// Bumped every time Baddel is paused or resumed, so the "paused" notice can be shown once
/// per pause rather than on every press of the shortcut.
static PAUSE_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Which pause (or stretch between pauses) this is.
pub fn pause_epoch() -> u64 {
    PAUSE_EPOCH.load(Ordering::SeqCst)
}

/// Applies the difference between two settings snapshots and tells every open
/// window about the new one. Returns the shortcuts macOS refused to bind.
pub fn apply(app: &AppHandle, previous: &Settings, next: &Settings) -> Vec<Binding> {
    if previous.paused != next.paused || previous.paused_until != next.paused_until {
        PAUSE_EPOCH.fetch_add(1, Ordering::SeqCst);
    }
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

/// The pause toggle, shared by the "Resume" item and the global shortcut. A pause it starts
/// lasts until the user resumes.
pub fn toggle_pause(app: &AppHandle) {
    let paused = app.state::<AppState>().get().paused;
    set_paused(app, !paused, None);
}

/// Pauses for `minutes`, or until the user resumes (`None`). Returns when a timed pause ends.
pub fn pause_for(app: &AppHandle, minutes: Option<u64>) -> Option<u64> {
    let until = minutes.map(|m| clock::now_ms() + m * 60_000);
    set_paused(app, true, until);
    until
}

fn set_paused(app: &AppHandle, paused: bool, until: Option<u64>) {
    let previous = app.state::<AppState>().get();
    let next = settings::update(app, |s| s.set_paused(paused, until));
    apply(app, &previous, &next);
}

/// Ends a timed pause whose time has come. Checked every couple of seconds (wall-clock time, so a
/// Mac that slept through the end resumes on waking) and again just before a conversion.
pub fn resume_if_expired(app: &AppHandle) {
    if app.state::<AppState>().get().pause_expired(clock::now_ms()) {
        set_paused(app, false, None);
    }
}
