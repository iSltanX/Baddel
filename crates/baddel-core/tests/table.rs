//! Runs the hand-derived fixture table (tests/fixtures/mod.rs) through the converter.

mod fixtures;

use baddel_core::LayoutMap;

#[test]
fn fixture_table() {
    let (mac, pc) = (LayoutMap::arabic_mac(), LayoutMap::arabic_pc());
    let mut failures = Vec::new();
    for &(layout, input, expected, note) in fixtures::CASES {
        let map = match layout {
            "mac" => &mac,
            "pc" => &pc,
            other => panic!("unknown layout {other:?} in fixtures"),
        };
        let got = map.convert(input).map(|c| c.text).unwrap_or_else(|| input.to_string());
        if got != expected {
            failures.push(format!("[{layout}] {input:?} → {got:?}, expected {expected:?}  ({note})"));
        }
    }
    assert!(failures.is_empty(), "{} of {} fixtures failed:\n{}", failures.len(), fixtures::CASES.len(), failures.join("\n"));
}

#[test]
fn fixture_table_is_complete() {
    assert!(fixtures::CASES.len() >= 60);
    assert!(fixtures::CASES.iter().any(|c| c.0 == "mac") && fixtures::CASES.iter().any(|c| c.0 == "pc"));
}
