use objc2::rc::autoreleasepool;
use objc2_app_kit::NSWorkspace;

/// Bundle identifier of the app that has keyboard focus.
pub fn bundle_id() -> Option<String> {
    autoreleasepool(|_| {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        Some(app.bundleIdentifier()?.to_string())
    })
}

/// Process id of the app that has keyboard focus.
pub fn pid() -> Option<i32> {
    autoreleasepool(|_| Some(NSWorkspace::sharedWorkspace().frontmostApplication()?.processIdentifier()))
}
