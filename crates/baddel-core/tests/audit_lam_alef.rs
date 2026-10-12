//! Audit (Copilot, 2026): lam-alef, round-trip, vote and protection edge cases.
//! Logic-level only — **not** proof of app behaviour.

use std::time::Instant;

use baddel_core::{detect_direction, is_protected, Direction, Layer, LayoutMap, LayoutProvider, StaticLayout, KEYCODES};

const A2L: Direction = Direction::ArabicToLatin;
const L2A: Direction = Direction::LatinToArabic;

fn mac() -> LayoutMap {
    LayoutMap::arabic_mac()
}
fn pc() -> LayoutMap {
    LayoutMap::arabic_pc()
}

/// Latin characters typed on Base/Shift of the ABC snapshot.
fn abc_chars() -> Vec<String> {
    let mut v = Vec::new();
    for k in KEYCODES {
        for l in Layer::PAIRED {
            if let Some(s) = StaticLayout::Abc.output(k, l) {
                if !s.is_empty() && !v.contains(&s) {
                    v.push(s);
                }
            }
        }
    }
    v
}

// ── Round trip, derived from the snapshots ───────────────────────────────────

/// Every single ABC key round-trips Latin → Arabic → Latin except the documented tatweel collision.
#[test]
fn single_key_round_trip_counterexamples_are_exactly_the_tatweel() {
    for (name, map, expected) in [("mac", mac(), vec!["_"]), ("pc", pc(), vec!["J"])] {
        let bad: Vec<String> = abc_chars()
            .into_iter()
            .filter(|s| map.convert_literal(&map.convert_literal(s, L2A), A2L) != *s)
            .collect();
        assert_eq!(bad, expected, "{name}: unexpected single-key round-trip failures");
    }
}

/// Seeded: random words of lowercase letters round-trip on Arabic (Mac), where لا is only g+h.
#[test]
fn lowercase_words_round_trip_on_mac() {
    let map = mac();
    let mut x: u64 = 0x5EED_2026;
    for _ in 0..20_000 {
        let mut w = String::new();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        for i in 0..(1 + x % 9) {
            w.push((b'a' + ((x >> (i * 5)) % 26) as u8) as char);
        }
        let back = map.convert_literal(&map.convert_literal(&w, L2A), A2L);
        assert_eq!(back, w, "mac round trip failed for {w:?} (seed state {x})");
    }
}

/// On Arabic – PC, «لا» typed with g+h comes back as `b` unless the gh-reading is an English word.
#[test]
fn pc_gh_round_trip_counterexample() {
    let map = pc();
    assert_eq!(map.convert_literal("gh", L2A), "لا");
    assert_eq!(map.convert_literal("لا", A2L), "b", "documented: lone لا reads as the single key");
    // With a dictionary verdict the g+h reading wins.
    assert_eq!(map.convert_literal(&map.convert_literal("night", L2A), A2L), "night");
    assert_eq!(map.convert_literal(&map.convert_literal("light", L2A), A2L), "light");
    // Non-word gh sequences do not come back.
    assert_eq!(map.convert_literal(&map.convert_literal("ghx", L2A), A2L), "bx");
}

// ── Lam-alef ─────────────────────────────────────────────────────────────────

#[test]
fn lam_alef_ligature_keys_on_pc() {
    let map = pc();
    assert_eq!(map.convert_literal("b", L2A), "لا");
    assert_eq!(map.convert_literal("G", L2A), "لأ");
    assert_eq!(map.convert_literal("B", L2A), "لآ");
    assert_eq!(map.convert_literal("T", L2A), "لإ");
    assert_eq!(map.convert_literal("لأ", A2L), "G");
    assert_eq!(map.convert_literal("لآ", A2L), "B");
    assert_eq!(map.convert_literal("لإ", A2L), "T");
}

#[test]
fn lam_alef_on_mac_is_always_two_keys() {
    let map = mac();
    assert_eq!(map.convert_literal("لا", A2L), "gh");
    assert_eq!(map.convert_literal("لأ", A2L), "gB");
    assert_eq!(map.convert_literal("لآ", A2L), "gH");
    assert_eq!(map.convert_literal("لإ", A2L), "gN");
}

/// Presentation-form ligatures (U+FEF5..FEFC) vote as Arabic but no key types them: they pass through.
#[test]
fn presentation_form_lam_alef_passes_through() {
    for map in [mac(), pc()] {
        assert_eq!(map.direction("ﻻ"), Some(A2L));
        assert_eq!(map.convert_as("ﻻ", A2L), "ﻻ");
    }
}

/// More than MAX_AMBIGUOUS_SITES (6) لا in one word: the search is skipped and every site reads `b`.
#[test]
fn pc_more_than_six_sites_reads_single_key() {
    let map = pc();
    let seven = "لا".repeat(7);
    assert_eq!(map.convert_literal(&seven, A2L), "b".repeat(7));
    let six = "لا".repeat(6);
    assert_eq!(map.convert_literal(&six, A2L), "b".repeat(6));
}

/// Worst case for the 2^n search: many words with exactly six sites each. Debug build budget.
#[test]
fn pc_disambiguation_cost_is_bounded() {
    let map = pc();
    let word = "لا".repeat(6);
    let text = vec![word.as_str(); 1_000].join(" "); // ~12k Arabic chars, 64 renders per word
    let t = Instant::now();
    let out = map.convert_as(&text, A2L);
    let el = t.elapsed();
    eprintln!("1000 words x 6 sites: {el:?}");
    assert_eq!(out.split(' ').count(), 1_000);
    assert!(el.as_secs_f64() < 5.0, "disambiguation took {el:?}");
}

/// 32 sites in one word: render(0) fallback, fine.
#[test]
fn pc_thirty_two_sites_in_one_word() {
    assert_eq!(pc().convert_as(&"لا".repeat(32), A2L), "b".repeat(32));
}

/// FAILING BY DESIGN — demonstrates BAD-001. With 33+ sites in one word, `render(0)` evaluates
/// `mask >> bit` with bit >= 32 on a u32 (disambiguate.rs:40): "attempt to shift right with
/// overflow" in any build with overflow checks (debug / `tauri dev` / tests). Release builds have
/// overflow checks off and happen to return the right text. Passes under `--release`.
#[test]
fn bad_001_thirty_three_sites_in_one_word_must_not_panic() {
    let long = "لا".repeat(5_000); // one 10k-char word
    let t = Instant::now();
    assert_eq!(pc().convert_as(&long, A2L), "b".repeat(5_000));
    assert!(t.elapsed().as_secs_f64() < 2.0);
    assert_eq!(pc().convert_as(&"لا".repeat(33), A2L), "b".repeat(33));
}

// ── Vote ─────────────────────────────────────────────────────────────────────

#[test]
fn tie_goes_to_arabic() {
    assert_eq!(detect_direction("ab اب"), Some(A2L));
    assert_eq!(detect_direction("abc اب"), Some(L2A));
}

/// BAD-002 (approved change, fix/audit-findings): this characterised Arabic-Indic digits and Arabic
/// punctuation voting Arabic; by the owner's decision only letters vote, as `detect_direction`
/// documents, so a digit typed on the Arabic layout no longer outvotes the word beside it.
#[test]
fn arabic_digits_and_punctuation_do_not_vote() {
    assert_eq!(detect_direction("١٢٣"), None);
    assert_eq!(detect_direction("؟؟"), None);
    assert_eq!(detect_direction("hello ١٢٣٤٥٦"), Some(L2A));
    assert_eq!(mac().convert("sg ١٢٣").unwrap().text, "سل ١٢٣");
    assert_eq!(mac().convert("اثممخ ١٢٣").unwrap().text, "hello 123");
    assert_eq!(detect_direction("123 ..."), None);
}

/// Technical words are left out of the vote: «اثممخ iPhone» is decided by the Arabic letters.
#[test]
fn technical_words_do_not_vote() {
    for map in [mac(), pc()] {
        let c = map.convert("اثممخ iPhone mp3").unwrap();
        assert_eq!(c.direction, A2L);
        assert_eq!(c.text, "hello iPhone mp3");
    }
}

// ── Protection edges ─────────────────────────────────────────────────────────

#[test]
fn protection_edges() {
    assert!(is_protected("https://example.com"));
    assert!(is_protected("(https://example.com)"));
    assert!(is_protected("WWW.example.com"));
    assert!(is_protected("mailto:a@b.com"));
    assert!(is_protected("a@b.com."));
    assert!(is_protected("~/Library"));
    assert!(is_protected("/usr/bin"));
    assert!(!is_protected("@handle"));
    assert!(!is_protected("a@b"));
    assert!(!is_protected("/"));
    assert!(!is_protected("//"));
    assert!(!is_protected("/1/2"));
}

/// Characterisation (BAD report): a link holding any Arabic letter (an IDN) is not protected, and
/// automatic conversion rewrites its Latin parts.
#[test]
fn idn_link_is_not_protected_and_gets_rewritten() {
    assert!(!is_protected("https://مثال.com"));
    let c = mac().convert("https://مثال.com").unwrap();
    assert_eq!(c.direction, L2A);
    assert_eq!(c.text, "اففحس://مثال.ذخو");
}

/// Technical-by-shape: any digit, or an inner capital that types no Arabic letter/mark.
#[test]
fn technical_edges() {
    let map = pc();
    assert_eq!(map.convert_as("x86_64", L2A), "x86_64");
    assert_eq!(map.convert_as("getElementById", L2A), "getElementById");
    // On Arabic (Mac) I types a shadda (a mark), so the same word is converted.
    assert_ne!(mac().convert_as("getElementById", L2A), "getElementById");
    // A leading capital alone is not technical.
    assert_ne!(map.convert_as("Hello", L2A), "Hello");
}
