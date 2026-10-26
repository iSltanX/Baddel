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

/// Whether the app runs from somewhere it should not stay: straight off a disk image, or from
/// the read-only copy Gatekeeper makes of a downloaded app (`…/AppTranslocation/…`). Either
/// way the permission and the updater would not survive.
///
/// A disk image mounts under `/Volumes/` read-only; an external drive mounts there too, but
/// writable, and an app installed on one is where it means to be. `mounts` is the output of
/// `/sbin/mount`.
pub fn is_temporary_location(bundle: &Path, mounts: &str) -> bool {
    if bundle.components().any(|part| part.as_os_str() == "AppTranslocation") {
        return true;
    }
    if !bundle.starts_with("/Volumes/") {
        return false;
    }
    // The volume the bundle lives on: the longest mount point that contains it.
    mounts
        .lines()
        .filter_map(|line| {
            let (_, rest) = line.split_once(" on ")?;
            let (point, options) = rest.rsplit_once(" (")?;
            Some((Path::new(point), options))
        })
        .filter(|(point, _)| bundle.starts_with(point))
        .max_by_key(|(point, _)| point.as_os_str().len())
        .is_some_and(|(_, options)| options.split(", ").any(|option| option.trim_end_matches(')') == "read-only"))
}

/// The system's list of mounted volumes, for [`is_temporary_location`].
pub fn mounts() -> String {
    Command::new("/sbin/mount")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

/// Opens a Finder window with `path` selected.
pub fn reveal_in_finder(path: &Path) {
    let _ = Command::new("/usr/bin/open").arg("-R").arg(path).spawn();
}

/// This user's own temporary folder (`/var/folders/…/T/`, readable by them alone), as the system
/// knows it — not `$TMPDIR`, which the process may have been given otherwise.
pub fn user_temp_dir() -> Option<PathBuf> {
    extern "C" {
        fn confstr(name: i32, buf: *mut std::ffi::c_char, len: usize) -> usize;
    }
    /// `_CS_DARWIN_USER_TEMP_DIR`.
    const USER_TEMP_DIR: i32 = 65537;
    let mut buf = vec![0u8; 1024];
    // SAFETY: the buffer is writable for its whole length; confstr NUL-terminates within it.
    let len = unsafe { confstr(USER_TEMP_DIR, buf.as_mut_ptr().cast(), buf.len()) };
    if len == 0 || len > buf.len() {
        return None;
    }
    buf.truncate(len - 1);
    String::from_utf8(buf).ok().map(PathBuf::from)
}

/// The user this process runs as, for launchd's `gui/<uid>` domain.
pub fn uid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    // SAFETY: no preconditions, cannot fail.
    unsafe { getuid() }
}

/// One key of a bundle's `Contents/Info.plist`, read from disk each time (an `NSBundle` would
/// answer from its cache, which an update has replaced the bundle under).
pub fn bundle_value(bundle: &Path, key: &str) -> Option<String> {
    let output = Command::new("/usr/bin/plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(bundle.join("Contents/Info.plist"))
        .output()
        .ok()?;
    let value = String::from_utf8(output.stdout).ok()?;
    (output.status.success() && !value.trim().is_empty()).then(|| value.trim().to_string())
}

/// Whether macOS opened Baddel as a login item, from the open-application event it sent: only
/// meaningful while that event is being handled, which is during setup. **Main thread.**
pub fn launched_as_login_item() -> bool {
    use objc2_foundation::NSAppleEventManager;
    /// `kAEOpenApplication`, `keyAEPropData` and `keyAELaunchedAsLogInItem` (four-char codes).
    const OPEN_APPLICATION: u32 = u32::from_be_bytes(*b"oapp");
    const PROP_DATA: u32 = u32::from_be_bytes(*b"prdt");
    const AS_LOGIN_ITEM: u32 = u32::from_be_bytes(*b"lgit");
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2_foundation::NSAppleEventDescriptor;
    let Some(event) = NSAppleEventManager::sharedAppleEventManager().currentAppleEvent() else { return false };
    // Sent directly: objc2-foundation types these through a crate Baddel does not otherwise need.
    // SAFETY: `eventID` returns a FourCharCode (UInt32) of a live descriptor.
    let id: u32 = unsafe { msg_send![&*event, eventID] };
    if id != OPEN_APPLICATION {
        return false;
    }
    // SAFETY: a keyword lookup (AEKeyword is a UInt32) that returns a descriptor or nil.
    let property: Option<Retained<NSAppleEventDescriptor>> = unsafe { msg_send![&*event, paramDescriptorForKeyword: PROP_DATA] };
    property.is_some_and(|p| {
        // SAFETY: `enumCodeValue` returns an OSType (UInt32).
        let code: u32 = unsafe { msg_send![&*p, enumCodeValue] };
        code == AS_LOGIN_ITEM
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTS: &str = "/dev/disk3s1s1 on / (apfs, sealed, local, read-only, journaled)
/dev/disk7s1 on /Volumes/Projects Drive (apfs, local, nodev, nosuid, journaled, noowners)
/dev/disk9s1 on /Volumes/Baddel 1.1.0 (apfs, local, nodev, nosuid, read-only, noowners, quarantine, mounted by me)
";

    #[test]
    fn a_disk_image_or_a_translocated_copy_is_temporary() {
        assert!(is_temporary_location(Path::new("/Volumes/Baddel 1.1.0/Baddel.app"), MOUNTS));
        assert!(is_temporary_location(
            Path::new("/private/var/folders/xy/T/AppTranslocation/0A1B-2C3D/d/Baddel.app"),
            MOUNTS
        ));
    }

    #[test]
    fn applications_home_folders_and_external_drives_are_not() {
        assert!(!is_temporary_location(Path::new("/Applications/Baddel.app"), MOUNTS));
        assert!(!is_temporary_location(Path::new("/Users/me/Applications/Baddel.app"), MOUNTS));
        // A writable external drive: someone keeps their apps (or their builds) there.
        assert!(!is_temporary_location(Path::new("/Volumes/Projects Drive/Apps/Baddel.app"), MOUNTS));
        // A folder that only mentions the word is not the system's translocation point.
        assert!(!is_temporary_location(Path::new("/Users/me/AppTranslocationNotes/Baddel.app"), MOUNTS));
    }
}
