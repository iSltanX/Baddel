//! Facts about the Mac itself, for diagnostics.

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
