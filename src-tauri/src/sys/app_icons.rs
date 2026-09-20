//! Resolving a bundle identifier to the name and icon macOS shows for that app.
//! **Main thread only** — call through [`super::on_main`].

use objc2::rc::autoreleasepool;
use objc2::AllocAnyThread;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSDeviceRGBColorSpace, NSGraphicsContext, NSImage, NSWorkspace,
};
use objc2_foundation::{NSDataBase64EncodingOptions, NSDictionary, NSPoint, NSRect, NSSize, NSString};

/// The icon is rendered at this many pixels square: enough for a crisp 32pt row on
/// a Retina display, small enough to hand to the webview as a data URL.
const ICON_PIXELS: usize = 64;

/// An installed app, as the exceptions list shows it.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    /// `data:image/png;base64,…`, or `None` when the app is not installed.
    pub icon: Option<String>,
}

/// Looks up an app by bundle identifier. An app that is no longer installed still
/// belongs in the list — it keeps its identifier as its name and gets no icon.
pub fn info(bundle_id: &str) -> AppInfo {
    match path_for(bundle_id) {
        Some(path) => AppInfo { id: bundle_id.to_string(), name: name_of(&path), icon: icon_of(&path) },
        None => AppInfo { id: bundle_id.to_string(), name: bundle_id.to_string(), icon: None },
    }
}

/// Reads an app bundle the user picked in the open panel.
pub fn info_at(path: &str) -> Option<AppInfo> {
    let id = bundle_id_at(path)?;
    Some(AppInfo { id, name: name_of(path), icon: icon_of(path) })
}

fn path_for(bundle_id: &str) -> Option<String> {
    autoreleasepool(|_| {
        let workspace = NSWorkspace::sharedWorkspace();
        let url = workspace.URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle_id))?;
        Some(url.path()?.to_string())
    })
}

/// The `CFBundleIdentifier` of the bundle at `path`, read from its `Info.plist`.
fn bundle_id_at(path: &str) -> Option<String> {
    let plist = std::path::Path::new(path).join("Contents/Info.plist");
    let output = std::process::Command::new("/usr/bin/plutil")
        .args(["-extract", "CFBundleIdentifier", "raw", "-o", "-"])
        .arg(&plist)
        .output()
        .ok()?;
    let id = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (output.status.success() && !id.is_empty()).then_some(id)
}

/// The name shown in Finder: the bundle's own file name without its extension.
fn name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map_or_else(|| path.to_string(), |stem| stem.to_string_lossy().into_owned())
}

fn icon_of(path: &str) -> Option<String> {
    autoreleasepool(|_| {
        let workspace = NSWorkspace::sharedWorkspace();
        let image = workspace.iconForFile(&NSString::from_str(path));
        let png = render_png(&image)?;
        Some(format!("data:image/png;base64,{png}"))
    })
}

/// Draws `image` into a fixed-size bitmap and encodes that as PNG.
///
/// Going through `TIFFRepresentation` instead would hand back every representation the
/// icon carries, up to 1024px — far more bytes than a list row needs.
fn render_png(image: &NSImage) -> Option<String> {
    // SAFETY: a null `planes` pointer asks AppKit to own the pixel buffer; the geometry
    // below describes a plain 8-bit RGBA bitmap, and `bytes_per_row`/`bits_per_pixel` of 0
    // let AppKit choose them.
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            ICON_PIXELS as isize,
            ICON_PIXELS as isize,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            0,
        )
    }?;

    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&context));
    let side = ICON_PIXELS as f64;
    image.drawInRect(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(side, side)));
    NSGraphicsContext::restoreGraphicsState_class();

    // SAFETY: the rep now holds drawn pixels, and PNG takes no required properties.
    let data = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
    Some(data.base64EncodedStringWithOptions(NSDataBase64EncodingOptions::empty()).to_string())
}
