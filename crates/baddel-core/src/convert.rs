use crate::disambiguate;
use crate::is_arabic;
use crate::layout::LayoutMap;

/// Which way a piece of text needs converting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Arabic characters were typed; Latin was meant.
    ArabicToLatin,
    /// Latin characters were typed; Arabic was meant.
    LatinToArabic,
}

impl Direction {
    pub fn reversed(self) -> Self {
        match self {
            Direction::ArabicToLatin => Direction::LatinToArabic,
            Direction::LatinToArabic => Direction::ArabicToLatin,
        }
    }
}

/// Result of [`LayoutMap::convert`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversion {
    pub text: String,
    pub direction: Direction,
}

/// Majority vote between Arabic-script characters and Latin letters.
/// `None` when the text has neither (digits, punctuation, whitespace only);
/// a tie counts as Arabic, since Arabic text legitimately embeds Latin words more often than the reverse.
pub fn detect_direction(text: &str) -> Option<Direction> {
    let (mut arabic, mut latin) = (0usize, 0usize);
    for c in text.chars() {
        if is_arabic(c) {
            arabic += 1;
        } else if c.is_alphabetic() {
            latin += 1;
        }
    }
    match (arabic, latin) {
        (0, 0) => None,
        (a, l) if a >= l => Some(Direction::ArabicToLatin),
        _ => Some(Direction::LatinToArabic),
    }
}

/// One matched key of the typed text.
pub(crate) struct Token<'a> {
    /// What the key maps to; `None` for characters no key of the map types (passed through).
    pub(crate) out: Option<&'a str>,
    pub(crate) src: &'a str,
    /// For a multi-character key whose characters are also typed by single keys:
    /// the other reading, those keys one by one ("لا" as `g`+`h` instead of `b`).
    pub(crate) alt: Option<String>,
}

impl LayoutMap {
    /// Detects the direction and converts. `None` when there is nothing to go on
    /// (no letters of either script) — the caller should leave the text alone.
    pub fn convert(&self, text: &str) -> Option<Conversion> {
        let direction = detect_direction(text)?;
        Some(Conversion { text: self.convert_as(text, direction), direction })
    }

    /// Converts in a known direction. Characters outside the map pass through unchanged.
    pub fn convert_as(&self, text: &str, direction: Direction) -> String {
        let mut out = String::with_capacity(text.len());
        // Word by word, so that the dictionary check in `disambiguate` sees whole words.
        for segment in split_keeping_whitespace(text) {
            if segment.starts_with(char::is_whitespace) {
                out.push_str(segment);
                continue;
            }
            let tokens = self.tokenize(segment, direction);
            if direction == Direction::ArabicToLatin && tokens.iter().any(|t| t.alt.is_some()) {
                out.push_str(&disambiguate::resolve(&tokens));
            } else {
                tokens.iter().for_each(|t| out.push_str(t.out.unwrap_or(t.src)));
            }
        }
        out
    }

    /// Greedy longest-match over the keys of the map.
    fn tokenize<'a>(&'a self, word: &'a str, direction: Direction) -> Vec<Token<'a>> {
        let table = self.table(direction);
        let bounds: Vec<usize> = word.char_indices().map(|(i, _)| i).chain([word.len()]).collect();
        let mut tokens = Vec::new();
        let mut i = 0;
        while i + 1 < bounds.len() {
            let longest = table.max_key_chars.min(bounds.len() - 1 - i).max(1);
            let hit = (1..=longest).rev().find_map(|n| {
                let src = &word[bounds[i]..bounds[i + n]];
                table.map.get(src).map(|out| (n, src, out.as_str()))
            });
            match hit {
                Some((n, src, out)) => {
                    let alt = (n > 1)
                        .then(|| {
                            src.chars()
                                .map(|c| table.map.get(c.encode_utf8(&mut [0; 4]) as &str).map(String::as_str))
                                .collect::<Option<String>>()
                        })
                        .flatten();
                    tokens.push(Token { out: Some(out), src, alt });
                    i += n;
                }
                None => {
                    tokens.push(Token { out: None, src: &word[bounds[i]..bounds[i + 1]], alt: None });
                    i += 1;
                }
            }
        }
        tokens
    }
}

/// Splits into alternating runs of whitespace and non-whitespace, losing nothing.
fn split_keeping_whitespace(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        let first = rest.chars().next()?;
        let is_ws = first.is_whitespace();
        let end = rest.find(|c: char| c.is_whitespace() != is_ws).unwrap_or(rest.len());
        let (head, tail) = rest.split_at(end);
        rest = tail;
        Some(head)
    })
}
