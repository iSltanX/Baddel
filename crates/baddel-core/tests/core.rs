//! Behavioural tests against the bundled macOS layout snapshots.

use baddel_core::{
    detect_direction, is_english_word, is_protected, Direction, Layer, LayoutMap, LayoutProvider, StaticLayout,
    KEYCODES,
};

fn mac() -> LayoutMap {
    LayoutMap::arabic_mac()
}
fn pc() -> LayoutMap {
    LayoutMap::arabic_pc()
}
fn to_latin(map: &LayoutMap, s: &str) -> String {
    map.convert_as(s, Direction::ArabicToLatin)
}
fn to_arabic(map: &LayoutMap, s: &str) -> String {
    map.convert_as(s, Direction::LatinToArabic)
}

// ── The headline cases ───────────────────────────────────────────────────────

#[test]
fn hello_on_both_layouts() {
    assert_eq!(mac().convert("اثممخ").unwrap().text, "hello");
    assert_eq!(pc().convert("اثممخ").unwrap().text, "hello");
}

#[test]
fn salam_on_both_layouts() {
    assert_eq!(mac().convert("sghl").unwrap().text, "سلام");
    assert_eq!(pc().convert("sghl").unwrap().text, "سلام");
}

#[test]
fn convert_reports_direction() {
    assert_eq!(mac().convert("اثممخ").unwrap().direction, Direction::ArabicToLatin);
    assert_eq!(mac().convert("sghl").unwrap().direction, Direction::LatinToArabic);
}

// ── Direction detection ──────────────────────────────────────────────────────

#[test]
fn direction_tie_counts_as_arabic() {
    assert_eq!(detect_direction("اب ab"), Some(Direction::ArabicToLatin));
}

#[test]
fn direction_mostly_arabic() {
    assert_eq!(detect_direction("اثممخ صخقمي ok"), Some(Direction::ArabicToLatin));
}

#[test]
fn direction_mostly_latin() {
    assert_eq!(detect_direction("sghl ugd;l و"), Some(Direction::LatinToArabic));
}

// ── Links, email addresses and paths ─────────────────────────────────────────

#[test]
fn links_emails_and_paths_are_protected() {
    for word in [
        "https://example.com",
        "http://x.org/a?b=c",
        "(https://example.com/path).",
        "www.example.com",
        "WWW.Example.com/1",
        "name@example.com",
        "first.last@mail.example.org,",
        "<name@example.com>",
        "~/Documents",
        "/usr/local/bin",
        "/Applications/Baddel.app",
    ] {
        assert!(is_protected(word), "{word:?}");
    }
}

#[test]
fn ordinary_and_wrongly_typed_words_are_not_protected() {
    for word in [
        "hello",
        "sghl",
        "ugd;l",
        // Arabic – PC: a lone "/" is «ظ», "." is «ز».
        "/gl",
        "/",
        "//",
        "z.d",
        "@handle",
        "name@",
        "name@example",
        "a@.com",
        // Anything holding an Arabic letter is text to convert, whatever it looks like.
        "اثممخ://",
        "صصصز.com",
        "123",
    ] {
        assert!(!is_protected(word), "{word:?}");
    }
}

#[test]
fn a_link_does_not_outvote_the_word_beside_it() {
    // Five Arabic letters against the link's dozen Latin ones: the link no longer counts.
    for map in [mac(), pc()] {
        let fixed = map.convert("اثممخ https://example.com").unwrap();
        assert_eq!(fixed.direction, Direction::ArabicToLatin);
        assert_eq!(fixed.text, "hello https://example.com");
    }
}

#[test]
fn an_email_survives_a_sentence_typed_on_the_wrong_layout() {
    // «ارسل الملف على name@example.com», typed entirely with the Latin layout on (Arabic – PC).
    let fixed = pc().convert("hvsg hglgt ugn name@example.com").unwrap();
    assert_eq!(fixed.direction, Direction::LatinToArabic);
    assert_eq!(fixed.text, "ارسل الملف على name@example.com");
}

#[test]
fn paths_are_kept_and_a_lone_slash_still_converts() {
    assert_eq!(to_arabic(&pc(), "hgltg ~/Documents/a.txt"), format!("{} ~/Documents/a.txt", to_arabic(&pc(), "hgltg")));
    assert_eq!(to_arabic(&pc(), "/gl"), "ظلم");
}

#[test]
fn explicit_direction_still_leaves_protected_words_alone() {
    for map in [mac(), pc()] {
        assert_eq!(to_latin(&map, "www.example.com/1"), "www.example.com/1");
        assert_eq!(to_arabic(&map, "name@example.com"), "name@example.com");
    }
}

// ── Technical words (iPhone, macOS, mp3) ─────────────────────────────────────

#[test]
fn technical_words_are_kept_where_the_vote_goes_latin_to_arabic() {
    // «سلام عليكم» typed with the Latin layout on, then a product name typed correctly.
    for map in [mac(), pc()] {
        let typed = "sghl ugd;l iPhone";
        let fixed = map.convert(typed).unwrap();
        assert_eq!(fixed.direction, Direction::LatinToArabic);
        assert_eq!(fixed.text, format!("{} iPhone", to_arabic(&map, "sghl ugd;l")));
        for word in ["macOS", "mp3", "x86_64", "v2", "H264", "utf8,"] {
            assert_eq!(to_arabic(&map, word), word, "{word:?}");
        }
    }
}

#[test]
fn a_technical_word_does_not_outvote_the_word_beside_it() {
    // Five Arabic letters against iPhone's six Latin ones: iPhone no longer counts.
    for map in [mac(), pc()] {
        let fixed = map.convert("اثممخ iPhone").unwrap();
        assert_eq!(fixed.direction, Direction::ArabicToLatin);
        assert_eq!(fixed.text, "hello iPhone");
    }
}

#[test]
fn capitals_that_type_hamza_or_marks_still_convert() {
    // Arabic – PC: Shift-H types «أ», so «سأل» typed with the Latin layout on is `sHg`.
    assert_eq!(to_arabic(&pc(), "sHg"), "سأل");
    // Arabic (Mac): Shift-B types «أ», Shift-Q a fatha.
    assert_eq!(to_arabic(&mac(), "sBg"), "سأل");
    assert_eq!(to_arabic(&mac(), "kQl"), to_arabic(&mac(), "k") + "\u{64e}" + &to_arabic(&mac(), "l"));
    // A capital at either end of the word is no sign: «أنا» starts with Shift-H on PC.
    assert_eq!(to_arabic(&pc(), "Hkh"), "أنا");
}

#[test]
fn which_capitals_mark_a_technical_word_depends_on_the_layout() {
    // `I` types ÷ on Arabic – PC and a shadda on Arabic (Mac).
    assert_eq!(to_arabic(&pc(), "getElementById"), "getElementById");
    assert_ne!(to_arabic(&mac(), "getElementById"), "getElementById");
}

#[test]
fn the_literal_conversion_keeps_nothing_back() {
    for map in [mac(), pc()] {
        assert_ne!(map.convert_literal("iPhone", Direction::LatinToArabic), "iPhone");
        assert_ne!(map.convert_literal("name@example.com", Direction::LatinToArabic), "name@example.com");
        // Otherwise the same conversion.
        assert_eq!(map.convert_literal("sghl ugd;l", Direction::LatinToArabic), to_arabic(&map, "sghl ugd;l"));
        assert_eq!(map.convert_literal("اثممخ", Direction::ArabicToLatin), "hello");
        // The other script is left as it is.
        assert_eq!(map.convert_literal("hello", Direction::ArabicToLatin), "hello");
    }
}

#[test]
fn only_protected_text_has_no_direction() {
    assert_eq!(detect_direction("https://example.com name@example.com"), None);
}

#[test]
fn direction_none_without_letters() {
    for s in ["", "   ", "123", "!?.,", "\n\t"] {
        assert_eq!(detect_direction(s), None, "{s:?}");
    }
}

#[test]
fn arabic_indic_digits_count_as_arabic() {
    assert_eq!(detect_direction("١٢٣"), Some(Direction::ArabicToLatin));
}

#[test]
fn convert_leaves_letterless_text_alone() {
    assert!(mac().convert("123 !?").is_none());
    assert!(mac().convert("").is_none());
}

#[test]
fn reversed_direction() {
    assert_eq!(Direction::ArabicToLatin.reversed(), Direction::LatinToArabic);
    assert_eq!(Direction::LatinToArabic.reversed(), Direction::ArabicToLatin);
}

// ── Pass-through ─────────────────────────────────────────────────────────────

#[test]
fn empty_string() {
    assert_eq!(to_latin(&mac(), ""), "");
    assert_eq!(to_arabic(&mac(), ""), "");
}

#[test]
fn whitespace_is_preserved_exactly() {
    assert_eq!(to_latin(&mac(), "  اثممخ \t صخقمي\n"), "  hello \t world\n");
}

#[test]
fn unmapped_characters_pass_through() {
    assert_eq!(to_latin(&mac(), "اثممخ 😀 €"), "hello 😀 €");
}

#[test]
fn return_tab_and_space_are_never_remapped() {
    for map in [mac(), pc()] {
        for s in [" ", "\t", "\n", "\r", "\u{a0}"] {
            assert_eq!(to_latin(&map, s), s);
            assert_eq!(to_arabic(&map, s), s);
        }
    }
}

// ── Digits and punctuation ───────────────────────────────────────────────────

#[test]
fn digits_map_both_ways() {
    assert_eq!(to_latin(&mac(), "فثسف ١٢٣"), "test 123");
    assert_eq!(to_arabic(&mac(), "2024"), "٢٠٢٤");
}

#[test]
fn parentheses_swap_like_the_keys_do() {
    // Shift-9 types "(" on ABC and ")" on the Arabic layouts.
    assert_eq!(to_latin(&mac(), ")اثممخ("), "(hello)");
    assert_eq!(to_latin(&pc(), ")اثممخ("), "(hello)");
}

#[test]
fn question_mark() {
    assert_eq!(to_latin(&mac(), "صاغ؟"), "why?");
    assert_eq!(to_arabic(&pc(), ";dt?"), "كيف؟");
}

#[test]
fn mac_comma_and_pc_comma_differ() {
    assert_eq!(to_arabic(&mac(), ","), "،");
    assert_eq!(to_arabic(&pc(), ","), "و");
}

// ── Shift layer ──────────────────────────────────────────────────────────────

#[test]
fn capital_letters_pc() {
    assert_eq!(to_latin(&pc(), "أثممخ"), "Hello");
    assert_eq!(to_arabic(&pc(), "Hello"), "أثممخ");
}

#[test]
fn capital_letters_mac() {
    assert_eq!(to_latin(&mac(), "آثممخ"), "Hello");
    assert_eq!(to_arabic(&mac(), "Hello"), "آثممخ");
}

#[test]
fn diacritics_come_from_the_shift_layer() {
    // fatha is Shift-Q on both layouts; kasra is Shift-A on PC and Shift-E on Mac.
    assert_eq!(to_latin(&pc(), "\u{64e}"), "Q");
    assert_eq!(to_latin(&pc(), "\u{650}"), "A");
    assert_eq!(to_latin(&mac(), "\u{650}"), "E");
}

#[test]
fn vocalised_arabic_roundtrips() {
    let word = "كَتَبَ";
    for map in [mac(), pc()] {
        assert_eq!(to_arabic(&map, &to_latin(&map, word)), word);
    }
}

// ── Layout differences ───────────────────────────────────────────────────────

#[test]
fn bottom_row_differs_between_layouts() {
    assert_eq!(to_arabic(&mac(), "zxcvbnm"), "ظطذدزرو");
    assert_eq!(to_arabic(&pc(), "zxcvbnm"), "ئءؤرلاىة");
}

#[test]
fn ta_marbuta_key() {
    // ة is the `]` key on Mac and the `m` key on PC.
    assert_eq!(to_arabic(&mac(), "lvns]"), "مدرسة");
    assert_eq!(to_arabic(&pc(), "l]vsm"), "مدرسة");
    assert_eq!(to_latin(&mac(), "مدرسة"), "lvns]");
}

// ── "لا": one key or two ─────────────────────────────────────────────────────

#[test]
fn pc_b_key_types_lam_alef() {
    assert_eq!(to_arabic(&pc(), "b"), "لا");
    assert_eq!(to_arabic(&pc(), "gh"), "لا");
}

#[test]
fn mac_has_no_lam_alef_key() {
    assert_eq!(to_arabic(&mac(), "b"), "ز");
    assert_eq!(to_latin(&mac(), "لا"), "gh");
    assert_eq!(to_latin(&mac(), "مهلاف"), "light");
    assert_eq!(to_latin(&mac(), "شزخعف"), "about");
}

#[test]
fn pc_lam_alef_prefers_a_real_word() {
    let pc = pc();
    assert_eq!(to_latin(&pc, "مهلاف"), "light");
    assert_eq!(to_latin(&pc, "ىهلاف"), "night");
    assert_eq!(to_latin(&pc, "اهلا"), "high");
    assert_eq!(to_latin(&pc, "شلاخعف"), "about");
    assert_eq!(to_latin(&pc, "فشلامث"), "table");
    assert_eq!(to_latin(&pc, "ىعةلاثق"), "number");
}

#[test]
fn pc_lam_alef_mixed_in_one_word() {
    // b·r·i·gh·t: the first لا is the B key, the second is G+H.
    assert_eq!(to_latin(&pc(), "لاقهلاف"), "bright");
}

#[test]
fn pc_lam_alef_defaults_to_b() {
    // Neither "xbz" nor "xghz" is a word → single key.
    assert_eq!(to_latin(&pc(), "ءلائ"), "xbz");
    // Both "bit"… only "bit" is a word anyway; and a bare لا is "b".
    assert_eq!(to_latin(&pc(), "لاهف"), "bit");
    assert_eq!(to_latin(&pc(), "لا"), "b");
}

#[test]
fn pc_lam_alef_is_decided_per_word() {
    assert_eq!(to_latin(&pc(), "شلاخعف فاث مهلاف"), "about the light");
}

#[test]
fn pc_lam_alef_ignores_surrounding_punctuation() {
    assert_eq!(to_latin(&pc(), ")مهلاف("), "(light)");
}

#[test]
fn pc_lam_alef_with_capital() {
    // Shift-L types "/" on PC, so "Light" is typed "/هلاف"; the word check ignores case.
    assert_eq!(to_latin(&pc(), "/هلاف"), "Light");
}

#[test]
fn option_layer_is_not_paired() {
    for map in [mac(), pc()] {
        for s in ["€", "°", "…", "“", "”", "ƒ", "پ", "گ"] {
            assert_eq!(to_latin(&map, s), s);
            assert_eq!(to_arabic(&map, s), s);
        }
    }
}

#[test]
fn pc_many_ambiguous_sites_fall_back_to_default() {
    let seven = "لا".repeat(7);
    assert_eq!(to_latin(&pc(), &seven), "b".repeat(7));
}

#[test]
fn pc_shifted_lam_alef_ligature_keys() {
    assert_eq!(to_arabic(&pc(), "G"), "لأ");
    assert_eq!(to_arabic(&pc(), "B"), "لآ");
    assert_eq!(to_arabic(&pc(), "T"), "لإ");
}

// ── Round trips ──────────────────────────────────────────────────────────────

#[test]
fn every_base_key_roundtrips() {
    for (arabic, map) in [(StaticLayout::ArabicMac, mac()), (StaticLayout::ArabicPc, pc())] {
        for keycode in KEYCODES {
            let (Some(en), Some(ar)) =
                (StaticLayout::Abc.output(keycode, Layer::Base), arabic.output(keycode, Layer::Base))
            else {
                continue;
            };
            if en == ar || en.chars().any(|c| c.is_control() || c.is_whitespace()) {
                continue;
            }
            assert_eq!(to_arabic(&map, &en), ar, "{arabic:?} key {keycode} forward");
            assert_eq!(to_latin(&map, &ar), en, "{arabic:?} key {keycode} back");
        }
    }
}

#[test]
fn sentences_roundtrip() {
    for map in [mac(), pc()] {
        for s in ["the quick fox jumps over a lazy dog", "open the file, then close it", "test 123"] {
            assert_eq!(to_latin(&map, &to_arabic(&map, s)), s);
        }
        for s in ["السلام عليكم ورحمة الله", "كتاب جديد عن البرمجة", "يوم ٢٥ شهر ١٢"] {
            assert_eq!(to_arabic(&map, &to_latin(&map, s)), s);
        }
    }
}

// ── Map construction ─────────────────────────────────────────────────────────

struct Nothing;
impl LayoutProvider for Nothing {
    fn output(&self, _: u16, _: Layer) -> Option<String> {
        None
    }
}

#[test]
fn empty_provider_gives_an_empty_map_that_changes_nothing() {
    let map = LayoutMap::build(&Nothing, &StaticLayout::Abc);
    assert!(map.is_empty());
    assert_eq!(map.convert_as("اثممخ", Direction::ArabicToLatin), "اثممخ");
}

#[test]
fn bundled_maps_are_not_empty() {
    assert!(!mac().is_empty() && !pc().is_empty());
}

struct Swapped;
impl LayoutProvider for Swapped {
    fn output(&self, keycode: u16, layer: Layer) -> Option<String> {
        // Base layer types what Shift types on Arabic–PC, and vice versa.
        let other = match layer {
            Layer::Base => Layer::Shift,
            Layer::Shift => Layer::Base,
            l => l,
        };
        StaticLayout::ArabicPc.output(keycode, other)
    }
}

#[test]
fn custom_provider_is_respected() {
    let map = LayoutMap::build(&Swapped, &StaticLayout::Abc);
    assert_eq!(map.convert_as("h", Direction::LatinToArabic), "أ");
    assert_eq!(map.convert_as("H", Direction::LatinToArabic), "ا");
}

#[test]
fn collisions_and_shared_symbols() {
    // On Arabic–PC "ـ" (tatweel) is both Shift-J and Shift-minus; the lower keycode is kept,
    // and "]" is Shift-D while the `]` key itself types "د".
    assert_eq!(to_latin(&pc(), "ظ"), "/");
    assert_eq!(to_latin(&pc(), "]"), "D");
    assert_eq!(to_arabic(&pc(), "]"), "د");
    // On Mac "ظ" is the `z` key.
    assert_eq!(to_latin(&mac(), "ظ"), "z");
}

#[test]
fn static_layout_out_of_range_keycode() {
    assert_eq!(StaticLayout::Abc.output(999, Layer::Base), None);
}

// ── Word list ────────────────────────────────────────────────────────────────

#[test]
fn word_list_lookup() {
    for w in ["light", "about", "Bright", "THROUGH", "(hello)", "world."] {
        assert!(is_english_word(w), "{w}");
    }
    for w in ["libt", "aghout", "", "123", "don't", "xbz"] {
        assert!(!is_english_word(w), "{w}");
    }
}

#[test]
fn word_list_is_sorted_lowercase_ascii() {
    let text = include_str!("../assets/en-words.txt");
    let mut prev = "";
    let mut n = 0;
    for w in text.lines() {
        assert!(!w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase()), "bad entry {w:?}");
        assert!(prev < w, "not strictly sorted at {w:?}");
        prev = w;
        n += 1;
    }
    assert!(n > 20_000, "word list too small: {n}");
}
