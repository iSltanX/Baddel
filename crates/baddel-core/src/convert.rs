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

/// Majority vote between Arabic and Latin letters.
/// `None` when the text has neither (digits, Arabic or Latin, punctuation and whitespace only);
/// a tie counts as Arabic, since Arabic text legitimately embeds Latin words more often than the reverse.
/// Letters of other scripts (中文, Привет, שלום) belong to neither side and do not vote.
///
/// Links, email addresses and file paths ([`is_protected`]) do not vote: a URL's dozen Latin
/// letters would otherwise outvote the short Arabic-typed word beside it. [`LayoutMap::direction`]
/// also leaves technical words out, which takes the layout to tell.
pub fn detect_direction(text: &str) -> Option<Direction> {
    vote(text, is_protected)
}

fn vote(text: &str, skip: impl Fn(&str) -> bool) -> Option<Direction> {
    let (mut arabic, mut latin) = (0usize, 0usize);
    for c in split_keeping_whitespace(text).filter(|word| !skip(word)).flat_map(str::chars) {
        // Letters (and Arabic marks) only: a digit or a «؟» shows which layout was on, not that it
        // was the wrong one.
        if !c.is_alphabetic() {
            continue;
        }
        if is_arabic(c) {
            arabic += 1;
        } else if is_latin_letter(c) {
            latin += 1;
        }
    }
    match (arabic, latin) {
        (0, 0) => None,
        (a, l) if a >= l => Some(Direction::ArabicToLatin),
        _ => Some(Direction::LatinToArabic),
    }
}

/// Whether a whitespace-delimited word is already right in Latin script and must be left exactly
/// as it is: a link (`://`, or starting with `www.`), an email address (`name@host.tld`), or a
/// file path (`~/…`, or `/…/…` with Latin letters between the first two slashes).
///
/// Wrongly typed Arabic cannot produce these: a word holding an Arabic letter is never protected,
/// `@` types `@` on both layouts, no Arabic word starts with the shadda that `~` types, and on
/// Arabic – PC a lone `/` is «ظ» — so the path rule asks for two.
pub fn is_protected(word: &str) -> bool {
    if word.chars().any(is_arabic) || !word.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    let core = word.trim_start_matches(['(', '[', '{', '<', '"', '\'', '«', '“', '‘']);
    is_link(core) || is_email(core) || is_path(core)
}

fn is_link(word: &str) -> bool {
    word.contains("://") || word.get(..4).is_some_and(|start| start.eq_ignore_ascii_case("www."))
}

fn is_email(word: &str) -> bool {
    let Some((local, domain)) = word.split_once('@') else { return false };
    let domain = domain.trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '}', '>', '"', '\'', '»', '”', '’']);
    let mut labels = domain.split('.');
    local.chars().any(char::is_alphanumeric)
        && domain.contains('.')
        && labels.all(|label| !label.is_empty() && !label.contains('@'))
}

fn is_path(word: &str) -> bool {
    if word.starts_with("~/") {
        return true;
    }
    word.strip_prefix('/')
        .and_then(|rest| rest.split_once('/'))
        .is_some_and(|(first, _)| first.chars().any(|c| c.is_ascii_alphabetic()))
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

/// Whether a word, typed as it is with the Latin layout, can only be a technical token — a name
/// like `iPhone` or `macOS`, or letters run together with digits (`mp3`, `x86_64`, `v2`) — and so
/// stays as it is. Never on the strength of a dictionary: only by shape, and only where wrongly
/// typed Arabic cannot take that shape.
///
/// - **Letters with digits.** The digit keys type digits on every Arabic layout, and an Arabic word
///   does not run into a number.
/// - **A capital inside a word** (between two letters) whose key types neither an Arabic letter
///   nor a mark on the paired Arabic layout. Shift types أ إ آ ؤ ئ ء and the vowel marks there,
///   and those land inside wrongly typed Arabic all the time (`sHg` is «سأل» on Arabic – PC); a
///   capital that types `[`, `؛` or `«` instead never does. That makes it depend on the layout:
///   `getElementById` is protected on Arabic – PC (I types ÷) but not on Arabic (I types a shadda).
fn is_technical(map: &LayoutMap, word: &str) -> bool {
    if word.chars().any(is_arabic) || !word.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    if word.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    let table = &map.table(Direction::LatinToArabic).map;
    let chars: Vec<char> = word.chars().collect();
    chars.windows(3).any(|w| {
        w[0].is_ascii_alphabetic() && w[1].is_ascii_uppercase() && w[2].is_ascii_alphabetic() && {
            let typed = table.get(w[1].encode_utf8(&mut [0; 4]) as &str);
            !typed.is_some_and(|out| out.chars().all(is_arabic_letter_or_mark))
        }
    })
}

/// Arabic letters (with the lam-alef ligatures a key may type) and the vowel marks — not the
/// Arabic punctuation (، ؛ ؟), digits or the tatweel, which a key may type too.
fn is_arabic_letter_or_mark(c: char) -> bool {
    matches!(c as u32, 0x0621..=0x063A | 0x0641..=0x065F | 0x0670 | 0xFEF5..=0xFEFC)
}

/// A letter of the Latin script: ASCII, and the accented letters of Latin-1 and the Latin
/// Extended blocks (café, Ærø, Đặng). Not Greek, Cyrillic, Hebrew or CJK.
fn is_latin_letter(c: char) -> bool {
    c.is_alphabetic() && matches!(c as u32, 0x0041..=0x005A | 0x0061..=0x007A | 0x00C0..=0x024F | 0x1E00..=0x1EFF)
}

/// Whether `text`, converted to `direction`'s script, came out as words of that script and nothing
/// else: every word at least two characters long, made of the script's letters (and, for Arabic,
/// its marks, beside at least one letter), and not one character over and over. «[]» typed with
/// ABC is «جد» on Arabic – PC and passes; «...» there is «ززز», a run of one key, and fails, as do
/// «؟» or «١٢٣», which are no letters at all.
fn reads_as_words(converted: &str, direction: Direction) -> bool {
    type Test = fn(char) -> bool;
    let (allowed, letter): (Test, Test) = match direction {
        Direction::LatinToArabic => {
            (is_arabic_letter_or_mark, |c| matches!(c as u32, 0x0621..=0x063A | 0x0641..=0x064A | 0xFEF5..=0xFEFC))
        }
        Direction::ArabicToLatin => (|c| c.is_ascii_alphabetic(), |c| c.is_ascii_alphabetic()),
    };
    let mut words = converted.split_whitespace().peekable();
    words.peek().is_some()
        && words.all(|word| {
            let first = word.chars().next();
            word.chars().count() >= 2
                && word.chars().all(allowed)
                && word.chars().any(letter)
                && word.chars().any(|c| Some(c) != first)
        })
}

impl LayoutMap {
    /// Detects the direction and converts. `None` when there is nothing to go on
    /// (no letters of either script) — the caller should leave the text alone.
    pub fn convert(&self, text: &str) -> Option<Conversion> {
        let direction = self.direction(text)?;
        Some(Conversion { text: self.convert_as(text, direction), direction })
    }

    /// [`convert`](Self::convert), for text typed with a known layout: `implied` is the direction
    /// that layout implies (typed with the Latin one, Arabic was meant). See
    /// [`direction_given`](Self::direction_given).
    pub fn convert_given(&self, text: &str, implied: Direction) -> Option<Conversion> {
        let direction = self.direction_given(text, implied)?;
        Some(Conversion { text: self.convert_as(text, direction), direction })
    }

    /// The majority vote of [`detect_direction`], with technical words left out as well as links:
    /// in «اثممخ iPhone» the five Arabic letters decide, not iPhone's six Latin ones.
    pub fn direction(&self, text: &str) -> Option<Direction> {
        vote(text, |word| self.keeps(word))
    }

    /// [`direction`](Self::direction) for text typed with a known layout. Letters decide whenever
    /// there are any. Text without a letter of either script is converted only when the layout it
    /// was typed with makes words of it: «[]» from ABC is «جد» on Arabic – PC, and «×،» from
    /// Arabic – PC is «OK». The same keys are no guess on their own — `[` is «ج» from ABC and «F»
    /// from Arabic – PC — so `implied` decides between them, never a vote. Punctuation, digits and
    /// symbols that stay punctuation, digits and symbols (`?`, `...`, `123`, `١٢٣`) are left as
    /// they are: `None`.
    pub fn direction_given(&self, text: &str, implied: Direction) -> Option<Direction> {
        if let Some(direction) = self.direction(text) {
            return Some(direction);
        }
        reads_as_words(&self.convert_as(text, implied), implied).then_some(implied)
    }

    /// Converts in a known direction. Characters outside the map pass through unchanged, and so do
    /// links, email addresses and file paths ([`is_protected`]) and technical words such as `iPhone`
    /// or `mp3` ([`is_technical`]).
    pub fn convert_as(&self, text: &str, direction: Direction) -> String {
        self.convert_words(text, direction, |word| self.keeps(word))
    }

    /// Converts everything that belongs to `direction`, leaving nothing out: what the explicit
    /// "Convert to Arabic / to English" commands do. They are the way to convert a word the
    /// protections would otherwise keep, so they apply none.
    pub fn convert_literal(&self, text: &str, direction: Direction) -> String {
        self.convert_words(text, direction, |_| false)
    }

    /// Whether a word is kept exactly as it is in automatic conversion.
    fn keeps(&self, word: &str) -> bool {
        is_protected(word) || is_technical(self, word)
    }

    fn convert_words(&self, text: &str, direction: Direction, keep: impl Fn(&str) -> bool) -> String {
        let mut out = String::with_capacity(text.len());
        // Word by word, so that the dictionary check in `disambiguate` sees whole words.
        for segment in split_keeping_whitespace(text) {
            if segment.starts_with(char::is_whitespace) || keep(segment) {
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
