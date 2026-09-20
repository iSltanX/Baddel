//! Hand-written bindings for the few Carbon (HIToolbox) symbols we need.
//! There is no maintained crate for the Text Input Sources API.
#![allow(non_upper_case_globals, non_snake_case)]

use std::ffi::c_void;

use core_foundation::array::CFArrayRef;
use core_foundation::dictionary::CFDictionaryRef;
use core_foundation::string::CFStringRef;

pub type TISInputSourceRef = *mut c_void;

pub const kUCKeyActionDown: u16 = 0;
pub const kUCKeyTranslateNoDeadKeysMask: u32 = 1;
/// Carbon modifier bits (`shiftKey`, `optionKey`), as used by `UCKeyTranslate` after `>> 8`.
pub const shiftKey: u32 = 0x0200;
pub const optionKey: u32 = 0x0800;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub static kTISPropertyInputSourceID: CFStringRef;
    pub static kTISPropertyLocalizedName: CFStringRef;
    pub static kTISPropertyInputSourceType: CFStringRef;
    pub static kTISPropertyUnicodeKeyLayoutData: CFStringRef;
    pub static kTISTypeKeyboardLayout: CFStringRef;

    pub fn TISCopyCurrentKeyboardLayoutInputSource() -> TISInputSourceRef;
    pub fn TISCreateInputSourceList(properties: CFDictionaryRef, include_all_installed: u8) -> CFArrayRef;
    pub fn TISGetInputSourceProperty(source: TISInputSourceRef, key: CFStringRef) -> *const c_void;
    pub fn TISSelectInputSource(source: TISInputSourceRef) -> i32;

    pub fn UCKeyTranslate(
        key_layout: *const c_void,
        virtual_key_code: u16,
        key_action: u16,
        modifier_key_state: u32,
        keyboard_type: u32,
        key_translate_options: u32,
        dead_key_state: *mut u32,
        max_string_length: usize,
        actual_string_length: *mut usize,
        unicode_string: *mut u16,
    ) -> i32;
    pub fn LMGetKbdType() -> u8;

    pub fn IsSecureEventInputEnabled() -> u8;
}
