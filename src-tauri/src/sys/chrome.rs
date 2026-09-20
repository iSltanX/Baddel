//! Window chrome measurements. **Main thread only** — call through [`super::on_main`].

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_app_kit::NSWindow;

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
