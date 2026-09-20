//! The Accessibility (TCC) permission.

use std::process::Command;

use accessibility_sys::{kAXTrustedCheckOptionPrompt, AXIsProcessTrustedWithOptions};
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;

/// Whether we may read and control other apps. With `prompt`, macOS shows its
/// own "would like to control this computer" dialog if we are not trusted yet.
pub fn is_trusted(prompt: bool) -> bool {
    // SAFETY: the key is a valid CFString constant; the dictionary outlives the call.
    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::from(prompt).as_CFType())]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
    }
}

/// Opens System Settings › Privacy & Security › Accessibility.
pub fn open_settings() {
    let _ = Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// Opens System Settings › Keyboard, where input sources are added.
pub fn open_keyboard_settings() {
    let _ = Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.Keyboard-Settings.extension")
        .spawn();
}

/// Password fields and some terminals turn this on; synthetic input must stay out.
pub fn secure_input_enabled() -> bool {
    // SAFETY: no preconditions.
    unsafe { super::carbon::IsSecureEventInputEnabled() != 0 }
}
