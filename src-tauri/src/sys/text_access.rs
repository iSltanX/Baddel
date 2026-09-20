//! Reading and replacing the selected text through the Accessibility API.

use std::ptr;

use accessibility_sys::{
    kAXErrorSuccess, kAXFocusedUIElementAttribute, kAXSelectedTextAttribute, kAXSelectedTextRangeAttribute,
    kAXStringForRangeParameterizedAttribute, kAXValueTypeCFRange, AXUIElementCopyAttributeValue,
    AXUIElementCopyParameterizedAttributeValue, AXUIElementCreateSystemWide, AXUIElementRef,
    AXUIElementSetAttributeValue, AXUIElementSetMessagingTimeout, AXValueCreate, AXValueGetValue, AXValueRef,
};
use core_foundation::base::{CFRange, CFType, TCFType};
use core_foundation::string::CFString;

/// Seconds to wait for the target app to answer an accessibility request.
const AX_TIMEOUT: f32 = 0.25;

pub enum Selection {
    Text(String),
    /// The element supports the attribute and nothing is selected.
    Empty,
    /// The element does not expose its selection (many Electron and web views).
    Unavailable,
}

/// The UI element with keyboard focus.
pub struct Focused(CFType);

impl Focused {
    pub fn get() -> Option<Self> {
        // SAFETY: create rule. The messaging timeout keeps a hung app from hanging us.
        let system = unsafe {
            let raw = AXUIElementCreateSystemWide();
            if raw.is_null() {
                return None;
            }
            AXUIElementSetMessagingTimeout(raw, AX_TIMEOUT);
            CFType::wrap_under_create_rule(raw.cast())
        };
        copy_attribute(&system, kAXFocusedUIElementAttribute).map(Focused)
    }

    pub fn selection(&self) -> Selection {
        match copy_attribute(&self.0, kAXSelectedTextAttribute).and_then(|v| v.downcast::<CFString>()) {
            Some(s) if s.to_string().is_empty() => Selection::Empty,
            Some(s) => Selection::Text(s.to_string()),
            None => Selection::Unavailable,
        }
    }

    /// The selection as (location, length) in UTF-16 units; length 0 is a bare caret.
    /// `None` when the element has no text range (then only the key-based path works).
    pub fn selected_range(&self) -> Option<(usize, usize)> {
        let value = copy_attribute(&self.0, kAXSelectedTextRangeAttribute)?;
        let mut range = CFRange { location: 0, length: 0 };
        // SAFETY: `value` is a live AXValue; on a type mismatch the call returns false and
        // leaves `range` untouched.
        let ok = unsafe {
            AXValueGetValue(value.as_CFTypeRef() as AXValueRef, kAXValueTypeCFRange, (&mut range as *mut CFRange).cast())
        };
        (ok && range.location >= 0 && range.length >= 0).then_some((range.location as usize, range.length as usize))
    }

    /// Selects (or, with length 0, places the caret at) a UTF-16 range.
    pub fn select_range(&self, location: usize, length: usize) -> bool {
        let Some(range) = ax_range(location, length) else { return false };
        let attribute = CFString::new(kAXSelectedTextRangeAttribute);
        // SAFETY: valid element and CF objects, all alive for the call.
        let status = unsafe {
            AXUIElementSetAttributeValue(
                self.0.as_CFTypeRef() as AXUIElementRef,
                attribute.as_concrete_TypeRef(),
                range.as_CFTypeRef(),
            )
        };
        status == kAXErrorSuccess
    }

    /// The text of a UTF-16 range of the element.
    pub fn string_for_range(&self, location: usize, length: usize) -> Option<String> {
        if length == 0 {
            return Some(String::new());
        }
        let range = ax_range(location, length)?;
        let attribute = CFString::new(kAXStringForRangeParameterizedAttribute);
        let mut value = ptr::null();
        // SAFETY: valid element and CF objects; on success `value` follows the create rule.
        let status = unsafe {
            AXUIElementCopyParameterizedAttributeValue(
                self.0.as_CFTypeRef() as AXUIElementRef,
                attribute.as_concrete_TypeRef(),
                range.as_CFTypeRef(),
                &mut value,
            )
        };
        if status != kAXErrorSuccess || value.is_null() {
            return None;
        }
        unsafe { CFType::wrap_under_create_rule(value) }.downcast::<CFString>().map(|s| s.to_string())
    }

    /// Replaces the selection. `true` only means the app accepted the request —
    /// some accept and ignore it, so the caller verifies.
    pub fn replace_selection(&self, text: &str) -> bool {
        let attribute = CFString::new(kAXSelectedTextAttribute);
        let value = CFString::new(text);
        // SAFETY: valid element and CF objects, all alive for the call.
        let status = unsafe {
            AXUIElementSetAttributeValue(
                self.0.as_CFTypeRef() as AXUIElementRef,
                attribute.as_concrete_TypeRef(),
                value.as_CFTypeRef(),
            )
        };
        status == kAXErrorSuccess
    }
}

fn ax_range(location: usize, length: usize) -> Option<CFType> {
    let range = CFRange { location: location as isize, length: length as isize };
    // SAFETY: `AXValueCreate` copies the CFRange it is given; create rule, null on failure.
    let value = unsafe { AXValueCreate(kAXValueTypeCFRange, (&range as *const CFRange).cast()) };
    (!value.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(value.cast()) })
}

fn copy_attribute(element: &CFType, attribute: &str) -> Option<CFType> {
    let attribute = CFString::new(attribute);
    let mut value = ptr::null();
    // SAFETY: valid element; on success `value` is set following the create rule.
    let status = unsafe {
        AXUIElementCopyAttributeValue(
            element.as_CFTypeRef() as AXUIElementRef,
            attribute.as_concrete_TypeRef(),
            &mut value,
        )
    };
    (status == kAXErrorSuccess && !value.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(value) })
}
