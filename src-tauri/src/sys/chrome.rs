//! Window chrome and app activation. **Main thread only** — call through [`super::on_main`].

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSWindow, NSWorkspace};

/// The height of a window's title bar, in points.
///
/// Tauri reports the same figure for a window's inner and outer size on macOS, so
/// growing a window to fit its page would otherwise lose the title bar's worth of
/// content off the bottom. AppKit knows the real split.
pub fn title_bar_height(ns_window: *mut c_void) -> f64 {
    let Some(window) = NonNull::new(ns_window.cast::<NSWindow>()) else { return 0.0 };
    // SAFETY: the pointer comes from Tauri's own live `NSWindow` for this window,
    // and is only read here, on the main thread, while that window exists.
    let window = unsafe { window.as_ref() };
    (window.frame().size.height - window.contentLayoutRect().size.height).max(0.0)
}

/// Gives the app's activation back to whatever the user was using.
///
/// tao calls `activateIgnoringOtherApps` as the app finishes launching, and Tauri has no
/// switch for it. A menu bar app has no window to come forward with, so macOS keeps the
/// request pending and honours it the first time *any* window appears — which would be
/// the conversion notice, pulling the whole app in front of the one being typed in.
/// A window that really wants focus asks for it itself, in `windows::focus`.
pub fn resign_activation() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(mtm);
    app.deactivate();
    if !app.isActive() {
        return;
    }
    // `deactivate` alone can leave the app active, holding the keys with no window to type
    // in (seen with a system permission prompt waiting on screen). The app the user is in
    // still owns the menu bar: hand it the keys directly.
    let Some(owner) = NSWorkspace::sharedWorkspace().menuBarOwningApplication() else { return };
    if owner.processIdentifier() != std::process::id() as i32 {
        owner.activateWithOptions(NSApplicationActivationOptions::empty());
    }
}

/// Brings the app forward for a dialog it shows with no window of its own (the update
/// prompts), so the alert does not open behind the app the user was in.
pub fn activate() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    // `activate()` needs macOS 14; the app supports 13.
    #[allow(deprecated)]
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
}

pub fn is_active() -> bool {
    MainThreadMarker::new().is_some_and(|mtm| NSApplication::sharedApplication(mtm).isActive())
}
