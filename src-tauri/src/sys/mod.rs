//! Everything that talks to macOS directly. All `unsafe` in the app lives under
//! this module, behind safe functions; the rest of the crate never sees a raw pointer.
//!
//! Threading: the Text Input Sources API (`input_source`) must be called on the
//! main thread — use [`on_main`]. Accessibility, CGEvent and NSPasteboard calls
//! are made from the conversion worker thread.

pub mod app_icons;
pub mod carbon;
pub mod chrome;
pub mod frontmost;
pub mod input_source;
pub mod keysynth;
pub mod pasteboard;
pub mod permissions;
pub mod sound;
pub mod text_access;

use std::sync::mpsc;
use std::time::Duration;

use tauri::AppHandle;

/// Runs `f` on the main thread and waits for its result (`None` on timeout).
pub fn on_main<T, F>(app: &AppHandle, f: F) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .ok()?;
    rx.recv_timeout(Duration::from_secs(1)).ok()
}
