//! Keyboard input sources: reading the live layouts and switching between them.
//! **Main thread only** — call through [`super::on_main`].

use std::ffi::c_void;

use baddel_core::{is_arabic, Layer, LayoutProvider};
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::data::CFData;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;

use super::carbon::*;

/// A keyboard layout copied out of the system: plain bytes, safe to move across threads.
#[derive(Clone)]
pub struct LiveLayout {
    pub id: String,
    /// The layout's `uchr` resource, as consumed by `UCKeyTranslate`.
    uchr: Vec<u8>,
    keyboard_type: u32,
}

impl LayoutProvider for LiveLayout {
    fn output(&self, keycode: u16, layer: Layer) -> Option<String> {
        let modifiers = match layer {
            Layer::Base => 0,
            Layer::Shift => shiftKey,
            Layer::Option => optionKey,
            Layer::OptionShift => optionKey | shiftKey,
        };
        let mut dead_keys = 0u32;
        let mut len = 0usize;
        let mut buf = [0u16; 8];
        // SAFETY: `uchr` holds a complete UCKeyboardLayout copied from the system and outlives
        // the call; `buf` is 8 UniChars long and we pass that as the maximum length.
        let status = unsafe {
            UCKeyTranslate(
                self.uchr.as_ptr().cast(),
                keycode,
                kUCKeyActionDown,
                (modifiers >> 8) & 0xFF,
                self.keyboard_type,
                kUCKeyTranslateNoDeadKeysMask,
                &mut dead_keys,
                buf.len(),
                &mut len,
                buf.as_mut_ptr(),
            )
        };
        (status == 0 && len > 0).then(|| String::from_utf16_lossy(&buf[..len.min(buf.len())]))
    }
}

/// The enabled layouts that matter to us, plus which script is active right now.
#[derive(Clone)]
pub struct Snapshot {
    pub arabic: Option<LiveLayout>,
    pub latin: Option<LiveLayout>,
    pub current_is_arabic: bool,
}

/// One enabled keyboard layout, for the layouts pane.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub arabic: bool,
}

/// Owned reference to a TIS input source.
struct Source(CFType);

impl Source {
    fn raw(&self) -> TISInputSourceRef {
        self.0.as_CFTypeRef() as TISInputSourceRef
    }

    fn id(&self) -> Option<String> {
        // SAFETY: valid source; the property, when present, is a CFString owned by the source.
        let ptr = unsafe { TISGetInputSourceProperty(self.raw(), kTISPropertyInputSourceID) };
        (!ptr.is_null()).then(|| unsafe { CFString::wrap_under_get_rule(ptr.cast()) }.to_string())
    }

    /// The name macOS shows for this layout in the input menu, in the user's language.
    fn name(&self) -> Option<String> {
        // SAFETY: valid source; the property, when present, is a CFString owned by the source.
        let ptr = unsafe { TISGetInputSourceProperty(self.raw(), kTISPropertyLocalizedName) };
        (!ptr.is_null()).then(|| unsafe { CFString::wrap_under_get_rule(ptr.cast()) }.to_string())
    }

    fn layout(&self) -> Option<LiveLayout> {
        // SAFETY: as above; the property is a CFData owned by the source. Input methods have none.
        let ptr = unsafe { TISGetInputSourceProperty(self.raw(), kTISPropertyUnicodeKeyLayoutData) };
        if ptr.is_null() {
            return None;
        }
        let data = unsafe { CFData::wrap_under_get_rule(ptr.cast()) };
        // SAFETY: no preconditions.
        let keyboard_type = unsafe { LMGetKbdType() } as u32;
        Some(LiveLayout { id: self.id()?, uchr: data.bytes().to_vec(), keyboard_type })
    }
}

/// Enabled keyboard layouts, in the order the user arranged them.
fn enabled_layouts() -> Vec<Source> {
    // SAFETY: the constants are valid CFStrings for the life of the process.
    let (key, value) = unsafe {
        (
            CFString::wrap_under_get_rule(kTISPropertyInputSourceType),
            CFString::wrap_under_get_rule(kTISTypeKeyboardLayout),
        )
    };
    let filter = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);
    // SAFETY: valid dictionary; the result follows the create rule (may be null).
    let list = unsafe { TISCreateInputSourceList(filter.as_concrete_TypeRef(), 0) };
    if list.is_null() {
        return Vec::new();
    }
    let list: CFArray<*const c_void> = unsafe { CFArray::wrap_under_create_rule(list) };
    list.iter()
        // SAFETY: every element is a live TISInputSource; retain it to outlive the array.
        .map(|item| Source(unsafe { CFType::wrap_under_get_rule(*item) }))
        .collect()
}

/// What a layout types for the `A` key tells us its script, whatever it is called.
fn types_arabic(layout: &LiveLayout) -> Option<bool> {
    let a = layout.output(0, Layer::Base)?;
    let c = a.chars().next()?;
    if is_arabic(c) {
        Some(true)
    } else if c.is_ascii_alphabetic() {
        Some(false)
    } else {
        None
    }
}

/// The enabled layouts we can map between, in the order macOS lists them.
pub fn list() -> Vec<Entry> {
    enabled_layouts()
        .iter()
        .filter_map(|source| {
            let layout = source.layout()?;
            let arabic = types_arabic(&layout)?;
            Some(Entry { name: source.name().unwrap_or_else(|| layout.id.clone()), id: layout.id, arabic })
        })
        .collect()
}

/// The layouts to convert between. `preferred_*` are input source ids chosen in the
/// settings; an empty or no-longer-enabled one falls back to the first of that script,
/// which is what "follow macOS" means.
pub fn snapshot(preferred_arabic: &str, preferred_latin: &str) -> Snapshot {
    let mut snap = Snapshot { arabic: None, latin: None, current_is_arabic: false };
    for layout in enabled_layouts().iter().filter_map(Source::layout) {
        let (wanted, slot) = match types_arabic(&layout) {
            Some(true) => (preferred_arabic, &mut snap.arabic),
            Some(false) => (preferred_latin, &mut snap.latin),
            None => continue,
        };
        // The chosen layout wins wherever it appears in the list; otherwise the first
        // of its script stands in for it. Ids are unique, so a choice already taken
        // is never overwritten.
        if layout.id == wanted || slot.is_none() {
            *slot = Some(layout);
        }
    }
    // SAFETY: create rule; null when no layout is active (should not happen).
    let current = unsafe { TISCopyCurrentKeyboardLayoutInputSource() };
    if !current.is_null() {
        let current = Source(unsafe { CFType::wrap_under_create_rule(current.cast_const()) });
        snap.current_is_arabic = current.layout().as_ref().and_then(types_arabic).unwrap_or(false);
    }
    snap
}

/// Makes the enabled layout with this id the active one.
pub fn select(id: &str) -> bool {
    enabled_layouts()
        .iter()
        .find(|s| s.id().as_deref() == Some(id))
        // SAFETY: valid, enabled source.
        .is_some_and(|s| unsafe { TISSelectInputSource(s.raw()) } == 0)
}
