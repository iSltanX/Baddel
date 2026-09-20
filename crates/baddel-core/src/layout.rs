use std::collections::HashMap;
use std::ops::RangeInclusive;

use crate::convert::Direction;
use crate::static_layouts::{self, Table};

/// macOS virtual keycodes of the printable keys on an ANSI/ISO keyboard.
pub const KEYCODES: RangeInclusive<u16> = 0..=50;

/// Modifier layer of a key. The order is the collision priority: when two keys
/// produce the same character, the one on the lower layer wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    Base,
    Shift,
    Option,
    OptionShift,
}

impl Layer {
    pub const ALL: [Layer; 4] = [Layer::Base, Layer::Shift, Layer::Option, Layer::OptionShift];

    /// The layers a [`LayoutMap`] pairs up. The Option layers are left out on purpose: nobody
    /// types English with Option held, and their symbols (… € ° “ ”) are typed deliberately on
    /// either layout — pairing them would turn a correct "…" into "گ".
    pub const PAIRED: [Layer; 2] = [Layer::Base, Layer::Shift];
}

/// What a keyboard layout types for a key. Implemented by the app over the
/// live system layouts, and by [`StaticLayout`] for the bundled snapshots.
pub trait LayoutProvider {
    /// The text produced by `keycode` on `layer`; `None` for dead or unmapped keys.
    /// May be more than one character (the Arabic–PC `B` key types "لا").
    fn output(&self, keycode: u16, layer: Layer) -> Option<String>;
}

/// Snapshots of the stock macOS layouts, generated from the system itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticLayout {
    /// "ABC" — the Latin side.
    Abc,
    /// "Arabic" — the macOS default Arabic layout (no لا key).
    ArabicMac,
    /// "Arabic – PC" — the Windows-style layout (`B` types "لا").
    ArabicPc,
}

impl StaticLayout {
    fn table(self) -> &'static Table {
        match self {
            StaticLayout::Abc => &static_layouts::ABC,
            StaticLayout::ArabicMac => &static_layouts::ARABIC_MAC,
            StaticLayout::ArabicPc => &static_layouts::ARABIC_PC,
        }
    }
}

impl LayoutProvider for StaticLayout {
    fn output(&self, keycode: u16, layer: Layer) -> Option<String> {
        let s = *self.table()[layer as usize].get(keycode as usize)?;
        (!s.is_empty()).then(|| s.to_string())
    }
}

/// One direction of the mapping, keyed by what was typed.
#[derive(Debug, Default)]
pub(crate) struct Table1 {
    pub(crate) map: HashMap<String, String>,
    /// Longest key, in chars — the tokenizer's look-ahead.
    pub(crate) max_key_chars: usize,
}

impl Table1 {
    fn insert_if_absent(&mut self, from: &str, to: &str) {
        if !self.map.contains_key(from) {
            self.max_key_chars = self.max_key_chars.max(from.chars().count());
            self.map.insert(from.to_string(), to.to_string());
        }
    }
}

/// Bidirectional character mapping between an Arabic and a Latin layout.
#[derive(Debug, Default)]
pub struct LayoutMap {
    ar_to_en: Table1,
    en_to_ar: Table1,
}

impl LayoutMap {
    /// Pairs up, key by key and layer by layer, what the two layouts type.
    pub fn build(arabic: &dyn LayoutProvider, latin: &dyn LayoutProvider) -> Self {
        let mut map = LayoutMap::default();
        for layer in Layer::PAIRED {
            for keycode in KEYCODES {
                let (Some(ar), Some(en)) = (arabic.output(keycode, layer), latin.output(keycode, layer))
                else {
                    continue;
                };
                // Keys that type the same thing on both layouts need no mapping,
                // and whitespace/control keys (Return, Tab, Space) must never be remapped.
                if ar == en || !is_mappable(&ar) || !is_mappable(&en) {
                    continue;
                }
                map.ar_to_en.insert_if_absent(&ar, &en);
                map.en_to_ar.insert_if_absent(&en, &ar);
            }
        }
        map
    }

    /// Bundled macOS "Arabic" ⇄ "ABC".
    pub fn arabic_mac() -> Self {
        Self::build(&StaticLayout::ArabicMac, &StaticLayout::Abc)
    }

    /// Bundled macOS "Arabic – PC" ⇄ "ABC".
    pub fn arabic_pc() -> Self {
        Self::build(&StaticLayout::ArabicPc, &StaticLayout::Abc)
    }

    /// True when the two layouts had nothing to pair (e.g. a provider failed).
    pub fn is_empty(&self) -> bool {
        self.ar_to_en.map.is_empty()
    }

    pub(crate) fn table(&self, direction: Direction) -> &Table1 {
        match direction {
            Direction::ArabicToLatin => &self.ar_to_en,
            Direction::LatinToArabic => &self.en_to_ar,
        }
    }
}

fn is_mappable(s: &str) -> bool {
    !s.is_empty() && !s.chars().any(|c| c.is_control() || c.is_whitespace())
}
