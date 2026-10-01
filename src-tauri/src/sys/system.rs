//! Facts about the Mac itself, for diagnostics, and where the app is running from.

use std::path::{Path, PathBuf};
use std::process::Command;

use objc2_foundation::NSProcessInfo;

/// The macOS version, as "27.0.1" (the patch is left off when it is zero, as `sw_vers` does).
pub fn os_version() -> String {
    let v = NSProcessInfo::processInfo().operatingSystemVersion();
    if v.patchVersion == 0 {
        format!("{}.{}", v.majorVersion, v.minorVersion)
    } else {
        format!("{}.{}.{}", v.majorVersion, v.minorVersion, v.patchVersion)
    }
}

/// The processor family, as Apple names it: "arm64" or "x86_64".
pub fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        other => other,
    }
}

/// The `.app` bundle this process runs from: three levels above `Contents/MacOS/<binary>`.
pub fn bundle_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bundle = exe.ancestors().nth(3)?;
    (bundle.extension()? == "app").then(|| bundle.to_path_buf())
}

/// Whether the app runs from somewhere it should not stay: straight off the disk image
/// (`/Volumes/…`), or from the read-only copy Gatekeeper makes of a downloaded app
/// (`…/AppTranslocation/…`). Either way the permission and the updater would not survive.
pub fn is_temporary_location(bundle: &Path) -> bool {
    bundle.starts_with("/Volumes/") || bundle.components().any(|part| part.as_os_str() == "AppTranslocation")
}

/// Opens a Finder window with `path` selected.
pub fn reveal_in_finder(path: &Path) {
    let _ = Command::new("/usr/bin/open").arg("-R").arg(path).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disk_image_or_a_translocated_copy_is_temporary() {
        assert!(is_temporary_location(Path::new("/Volumes/Baddel/Baddel.app")));
        assert!(is_temporary_location(Path::new(
            "/private/var/folders/xy/T/AppTranslocation/0A1B-2C3D/d/Baddel.app"
        )));
    }

    #[test]
    fn applications_and_home_folders_are_not() {
        assert!(!is_temporary_location(Path::new("/Applications/Baddel.app")));
        assert!(!is_temporary_location(Path::new("/Users/me/Applications/Baddel.app")));
        // A folder that only mentions the word is not the system's translocation point.
        assert!(!is_temporary_location(Path::new("/Users/me/AppTranslocationNotes/Baddel.app")));
    }
}
