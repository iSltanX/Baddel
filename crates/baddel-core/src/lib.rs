//! Keyboard-layout text conversion (Arabic ⇄ Latin).
//!
//! Fixes text typed while the wrong keyboard layout was active: every character
//! is mapped back through the physical key that produced it to what that key
//! produces on the other layout. This is a per-key substitution, not
//! transliteration.
//!
//! Pure Rust: no Tauri, no macOS APIs. The live keyboard layouts are injected
//! by the app through [`LayoutProvider`]; [`StaticLayout`] ships snapshots of
//! the stock macOS layouts as a fallback and for tests.
//!
//! ```
//! use baddel_core::{Direction, LayoutMap};
//!
//! let map = LayoutMap::arabic_mac();
//! let fixed = map.convert("اثممخ").unwrap();
//! assert_eq!(fixed.text, "hello");
//! assert_eq!(fixed.direction, Direction::ArabicToLatin);
//! assert_eq!(map.convert("sghl").unwrap().text, "سلام");
//! ```

mod convert;
mod disambiguate;
mod layout;
mod static_layouts;

pub use convert::{detect_direction, Conversion, Direction};
pub use disambiguate::is_english_word;
pub use layout::{Layer, LayoutMap, LayoutProvider, StaticLayout, KEYCODES};

/// Whether `c` belongs to an Arabic script block (letters, marks, digits, presentation forms).
pub fn is_arabic(c: char) -> bool {
    matches!(
        c as u32,
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_arabic_script() {
        assert!("اثممخ".chars().all(is_arabic));
        assert!(!"hello".chars().any(is_arabic));
        assert!(is_arabic('ﻻ')); // presentation-form lam-alef
        assert!(is_arabic('٣')); // Arabic-Indic digit
        assert!(!is_arabic(' ') && !is_arabic('1'));
    }
}
