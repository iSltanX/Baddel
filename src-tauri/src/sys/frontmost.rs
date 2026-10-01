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
