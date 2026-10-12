//! Audit (Copilot, 2026): seeded generative tests over the public API of baddel-core.
//!
//! These exercise the pure conversion logic only. They are **not** proof of app behaviour
//! (no clipboard, no key events, no live macOS layouts).
//!
//! Re-run a failing case with `BADDEL_AUDIT_SEED=<seed> cargo test -p baddel-core --test audit_generative`.

use std::panic::{catch_unwind, AssertUnwindSafe};

use baddel_core::{detect_direction, is_protected, Direction, LayoutMap};

const CASES_PER_LAYOUT: usize = 10_000;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

fn base_seed() -> u64 {
    std::env::var("BADDEL_AUDIT_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(0x00BA_DDE1_2026)
}

const WHITESPACE: &[&str] = &[" ", "  ", "\t", "\n", "\r\n", "\u{00A0}", "\u{3000}", " \t\n"];
/// Characters no key of either layout types: they must pass through untouched, in order.
const FOREIGN: &[char] = &['中', '文', '😀', '👩', '\u{200D}', '\u{1F3FD}', 'é', '\u{0301}', '\u{202E}', '\u{2066}', 'Ж', '€', '\u{FEFF}'];

fn ascii_printable(rng: &mut Rng) -> char {
    (0x21 + rng.below(0x7f - 0x21) as u8) as char
}
fn arabic(rng: &mut Rng) -> char {
    // Letters, marks, digits, punctuation, tatweel, and presentation-form lam-alef.
    let ranges: &[(u32, u32)] = &[(0x0621, 0x064A), (0x064B, 0x0652), (0x0660, 0x0669), (0x060C, 0x060C), (0x061B, 0x061F), (0x0640, 0x0640), (0xFEF5, 0xFEFC)];
    let (lo, hi) = *rng.pick(ranges);
    char::from_u32(lo + rng.below((hi - lo + 1) as usize) as u32).unwrap_or('ا')
}

fn gen_text(rng: &mut Rng) -> String {
    let mut s = String::new();
    let words = rng.below(8);
    for i in 0..words {
        if i > 0 || rng.below(4) == 0 {
            s.push_str(rng.pick(WHITESPACE));
        }
        let len = 1 + rng.below(12);
        let flavour = rng.below(5);
        for _ in 0..len {
            let c = match (flavour, rng.below(10)) {
                (_, 0) => *rng.pick(FOREIGN),
                (0, _) => ascii_printable(rng),
                (1, _) => arabic(rng),
                (2, n) if n < 5 => ascii_printable(rng),
                (2, _) => arabic(rng),
                (3, _) => *rng.pick(&['g', 'h', 'b', 'G', 'H', 'B', 'T', 'N', 'Y', 'ل', 'ا', 'أ', 'إ', 'آ']),
                _ => *rng.pick(&[':', '/', '@', '.', 'w', 'h', 't', 'p', 's', '~', 'm', 'ا', '1', 'X']),
            };
            s.push(c);
        }
        if rng.below(6) == 0 {
            s.push_str(rng.pick(WHITESPACE));
        }
    }
    s
}

/// Splits into alternating whitespace / non-whitespace runs, as the converter does.
fn runs(s: &str) -> Vec<(bool, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut prev: Option<bool> = None;
    for (i, c) in s.char_indices() {
        let ws = c.is_whitespace();
        if prev.is_some_and(|p| p != ws) {
            out.push((prev.unwrap(), &s[start..i]));
            start = i;
        }
        prev = Some(ws);
    }
    if let Some(p) = prev {
        out.push((p, &s[start..]));
    }
    out
}

fn check_one(map: &LayoutMap, text: &str) {
    let dirs = [Direction::ArabicToLatin, Direction::LatinToArabic];
    let in_runs = runs(text);
    let in_foreign: Vec<char> = text.chars().filter(|c| FOREIGN.contains(c)).collect();

    for d in dirs {
        for out in [map.convert_as(text, d), map.convert_literal(text, d)] {
            let out_runs = runs(&out);
            // Whitespace runs survive byte-for-byte, in place, and words map one-to-one.
            let ws_in: Vec<_> = in_runs.iter().filter(|r| r.0).map(|r| r.1).collect();
            let ws_out: Vec<_> = out_runs.iter().filter(|r| r.0).map(|r| r.1).collect();
            assert_eq!(ws_in, ws_out, "whitespace changed ({d:?}): {text:?} -> {out:?}");
            assert_eq!(
                in_runs.iter().map(|r| r.0).collect::<Vec<_>>(),
                out_runs.iter().map(|r| r.0).collect::<Vec<_>>(),
                "word/space structure changed ({d:?}): {text:?} -> {out:?}"
            );
            // Characters no key types pass through unchanged and in order.
            let out_foreign: Vec<char> = out.chars().filter(|c| FOREIGN.contains(c)).collect();
            assert_eq!(in_foreign, out_foreign, "foreign chars changed ({d:?}): {text:?} -> {out:?}");
        }
        // Protected words are kept exactly in automatic conversion.
        let out = map.convert_as(text, d);
        for ((ws, a), (_, b)) in in_runs.iter().zip(runs(&out).iter()) {
            if !ws && is_protected(a) {
                assert_eq!(a, b, "protected word changed ({d:?}) in {text:?}");
            }
        }
    }
    // convert() is direction() followed by convert_as().
    match (map.direction(text), map.convert(text)) {
        (None, None) => {}
        (Some(d), Some(c)) => {
            assert_eq!(c.direction, d);
            assert_eq!(c.text, map.convert_as(text, d));
        }
        other => panic!("convert/direction disagree on {text:?}: {other:?}"),
    }
    // Detection without a layout never panics either.
    let _ = detect_direction(text);
}

fn run_layout(name: &str, map: &LayoutMap, salt: u64) {
    let seed0 = base_seed() ^ salt;
    let start = std::time::Instant::now();
    for i in 0..CASES_PER_LAYOUT {
        let seed = seed0.wrapping_add(i as u64).max(1);
        let mut rng = Rng(seed);
        for _ in 0..4 {
            rng.next();
        }
        let text = gen_text(&mut rng);
        let r = catch_unwind(AssertUnwindSafe(|| check_one(map, &text)));
        if let Err(e) = r {
            let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
            panic!("[{name}] case seed={seed} text={text:?}: {}", msg.unwrap_or_default());
        }
    }
    eprintln!("[{name}] {CASES_PER_LAYOUT} cases in {:?} (base seed {})", start.elapsed(), base_seed());
}

#[test]
fn generative_invariants_arabic_mac() {
    run_layout("mac", &LayoutMap::arabic_mac(), 0x1111);
}

#[test]
fn generative_invariants_arabic_pc() {
    run_layout("pc", &LayoutMap::arabic_pc(), 0x2222);
}
