//! The app itself as a login item, through `SMAppService.mainApp` (macOS 13 and later): the one
//! entry System Settings › General › Login Items lists for Baddel, which the user can turn off
//! there. Any thread.
//!
//! Also reads what macOS thinks of an older Baddel's LaunchAgent (`statusForLegacyURL:`), so the
//! move away from it can tell an agent the user switched off from one still allowed.

use std::path::Path;

use objc2::msg_send;
use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::{AnyClass, AnyObject};
use objc2_foundation::{NSError, NSString, NSURL};

// The class lives in ServiceManagement, which nothing else links.
#[link(name = "ServiceManagement", kind = "framework")]
extern "C" {}

/// `SMAppServiceStatus`, as Apple numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not registered, or unregistered since (by Baddel, or by the user in System Settings).
    NotRegistered,
    /// Registered, and allowed to launch at login.
    Enabled,
    /// Registered, but the user has to allow it in System Settings (or switched it off there).
    RequiresApproval,
    /// macOS could not find the service: no bundle, or ServiceManagement is unavailable.
    NotFound,
}

impl Status {
    fn from_raw(raw: isize) -> Self {
        match raw {
            0 => Status::NotRegistered,
            1 => Status::Enabled,
            2 => Status::RequiresApproval,
            _ => Status::NotFound,
        }
    }
}

/// What `register` or `unregister` reported: the `NSError` code (one of the `kSMError…` values
/// below for ServiceManagement's own errors) and its domain, for diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: i64,
    pub domain: String,
}

/// `kSMErrorAlreadyRegistered`.
pub const ALREADY_REGISTERED: i64 = 12;
/// `kSMErrorJobNotFound`: unregistering what is not registered.
pub const JOB_NOT_FOUND: i64 = 6;
/// `kSMErrorLaunchDeniedByUser`.
pub const LAUNCH_DENIED_BY_USER: i64 = 11;
/// `kSMErrorInvalidSignature`.
pub const INVALID_SIGNATURE: i64 = 3;

fn class() -> Option<&'static AnyClass> {
    AnyClass::get(c"SMAppService")
}

fn main_app() -> Option<Retained<AnyObject>> {
    // SAFETY: `+[SMAppService mainAppService]` takes nothing and returns the shared object.
    unsafe { msg_send![class()?, mainAppService] }
}

pub fn status() -> Status {
    autoreleasepool(|_| {
        let Some(service) = main_app() else { return Status::NotFound };
        // SAFETY: `status` is a plain NSInteger property of a live service object.
        let raw: isize = unsafe { msg_send![&*service, status] };
        Status::from_raw(raw)
    })
}

pub fn register() -> Result<(), Error> {
    call(|service| {
        // SAFETY: `registerAndReturnError:` on a live service; objc2 manages the error out-pointer.
        unsafe { msg_send![service, registerAndReturnError: _] }
    })
}

pub fn unregister() -> Result<(), Error> {
    call(|service| {
        // SAFETY: as for `register`.
        unsafe { msg_send![service, unregisterAndReturnError: _] }
    })
}

fn call(f: impl FnOnce(&AnyObject) -> Result<(), Retained<NSError>>) -> Result<(), Error> {
    autoreleasepool(|_| {
        let Some(service) = main_app() else { return Err(Error { code: -1, domain: "SMAppService".into() }) };
        f(&service).map_err(|error| Error { code: error.code() as i64, domain: error.domain().to_string() })
    })
}

/// What macOS has decided for an older login item it found in a LaunchAgents folder.
pub fn legacy_status(plist: &Path) -> Status {
    autoreleasepool(|_| {
        let Some(class) = class() else { return Status::NotFound };
        let url = NSURL::fileURLWithPath(&NSString::from_str(&plist.to_string_lossy()));
        // SAFETY: `+statusForLegacyURL:` takes a file URL and returns an NSInteger.
        let raw: isize = unsafe { msg_send![class, statusForLegacyURL: &*url] };
        Status::from_raw(raw)
    })
}

/// System Settings › General › Login Items.
pub fn open_settings() {
    autoreleasepool(|_| {
        if let Some(class) = class() {
            // SAFETY: a class method that takes nothing and returns nothing.
            let () = unsafe { msg_send![class, openSystemSettingsLoginItems] };
        }
    })
}
