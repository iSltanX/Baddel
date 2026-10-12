use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::window::{
    copy_window_info, kCGNullWindowID, kCGWindowLayer, kCGWindowListExcludeDesktopElements,
    kCGWindowListOptionOnScreenOnly, kCGWindowOwnerPID,
};
use objc2::rc::autoreleasepool;
use objc2_app_kit::NSWorkspace;

/// The app that has keyboard focus, as a conversion remembers it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct App {
    pub pid: Option<i32>,
    pub bundle_id: Option<String>,
    /// The name macOS shows for it ("Mail"), for the notices that name the app.
    pub name: Option<String>,
}

/// The frontmost app, all three facts read at once.
pub fn current() -> App {
    autoreleasepool(|_| {
        let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else { return App::default() };
        App {
            pid: Some(app.processIdentifier()),
            bundle_id: app.bundleIdentifier().map(|id| id.to_string()),
            name: app.localizedName().map(|name| name.to_string()),
        }
    })
}

/// Process id of the app that has keyboard focus.
pub fn pid() -> Option<i32> {
    autoreleasepool(|_| Some(NSWorkspace::sharedWorkspace().frontmostApplication()?.processIdentifier()))
}

/// The process whose window is in front on screen, as the window server has it now. Synthetic
/// keys go to that app; `NSWorkspace.frontmostApplication`, read off the main thread, went on
/// naming the app that had been in front for over a second after a switch (device test S2).
/// `None` when no ordinary window is on screen.
pub fn front_window_pid() -> Option<i32> {
    let windows = copy_window_info(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID)?;
    // SAFETY: CoreGraphics' own constant keys, alive for the process.
    let (layer, owner) = unsafe { (CFString::wrap_under_get_rule(kCGWindowLayer), CFString::wrap_under_get_rule(kCGWindowOwnerPID)) };
    let number = |window: &CFDictionary<CFString, CFType>, key: &CFString| {
        window.find(key).and_then(|value| value.downcast::<CFNumber>()).and_then(|n| n.to_i32())
    };
    // Front to back; layer 0 holds ordinary windows, so menus, panels and notices are passed over.
    windows.iter().find_map(|item| {
        // SAFETY: every item of the list is a window's CFDictionary, alive with the list.
        let window: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_get_rule(*item as CFDictionaryRef) };
        (number(&window, &layer) == Some(0)).then(|| number(&window, &owner)).flatten()
    })
}

/// The bundle identifier of the running app with this process id.
pub fn bundle_id_of(pid: i32) -> Option<String> {
    autoreleasepool(|_| {
        let apps = NSWorkspace::sharedWorkspace().runningApplications();
        let app = apps.iter().find(|app| app.processIdentifier() == pid)?;
        app.bundleIdentifier().map(|id| id.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the running apps only; no input is sent.
    #[test]
    fn the_app_in_front_is_found_by_its_pid() {
        let front = current();
        if let Some(pid) = front.pid {
            assert_eq!(bundle_id_of(pid), front.bundle_id);
        }
        assert_eq!(bundle_id_of(-1), None);
    }

    /// Reads the window list only; no input is sent.
    #[test]
    fn the_front_window_belongs_to_a_running_app() {
        if let Some(pid) = front_window_pid() {
            assert!(pid > 0);
            assert!(NSWorkspace::sharedWorkspace().runningApplications().iter().any(|app| app.processIdentifier() == pid));
        }
    }
}
