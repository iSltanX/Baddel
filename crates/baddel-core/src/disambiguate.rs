//! Resolves keys that type more than one character.
//!
//! On the Arabic–PC layout `B` types "لا", which is also what `G` then `H`
//! type. Going back to Latin, "مهلاف" is either "libt" or "light". We try each
//! reading of the word against an English word list and keep the first real
//! word; with no verdict, the single key wins (`b` is far more frequent than `gh`).

use std::sync::OnceLock;

use crate::convert::Token;

/// Beyond this many ambiguous spots in one word, skip the search (2^n readings).
const MAX_AMBIGUOUS_SITES: usize = 6;

static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();

fn words() -> &'static [&'static str] {
    // Sorted, lowercase a–z, one per line — see THIRD_PARTY.md.
    WORDS.get_or_init(|| include_str!("../assets/en-words.txt").lines().collect())
}

/// Whether `word`, ignoring case and surrounding punctuation, is a known English word.
pub fn is_english_word(word: &str) -> bool {
    let core = word.trim_matches(|c: char| !c.is_ascii_alphabetic());
    if core.is_empty() || !core.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    let lower = core.to_ascii_lowercase();
    words().binary_search(&lower.as_str()).is_ok()
}

/// Joins the tokens of one word, choosing between `out` and `alt` where both exist.
pub(crate) fn resolve(tokens: &[Token<'_>]) -> String {
    let sites: Vec<usize> =
        tokens.iter().enumerate().filter(|(_, t)| t.alt.is_some()).map(|(i, _)| i).collect();

    let render = |mask: u32| -> String {
        let mut s = String::new();
        for (i, t) in tokens.iter().enumerate() {
            let use_alt = sites.iter().position(|&site| site == i).is_some_and(|bit| mask >> bit & 1 == 1);
            match (&t.alt, use_alt) {
                (Some(alt), true) => s.push_str(alt),
                _ => s.push_str(t.out.unwrap_or(t.src)),
            }
        }
        s
    };

    if sites.len() > MAX_AMBIGUOUS_SITES {
        return render(0);
    }
    // Mask 0 (every site read as the single key) is tried first, so it also wins ties.
    (0..1u32 << sites.len()).map(render).find(|c| is_english_word(c)).unwrap_or_else(|| render(0))
}
