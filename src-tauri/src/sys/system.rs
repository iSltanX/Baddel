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
