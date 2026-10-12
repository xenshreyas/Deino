//! The rename shortcut: "Niall is spelled Nyal", "rename Acme to Acme
//! Corp everywhere" and the like are done without the AI.
//!
//! A request is a plain rename only when the whole of it matches one of the
//! patterns and both names are short (at most [`MAX_NAME_WORDS`] words, not
//! starting with an article or a pointer such as "the" or "this", which
//! describe a part of the notes rather than name something). Every
//! whole-word occurrence is replaced, ignoring case, in text runs (a link's
//! text included, never its target) and in mention labels (never the person
//! they point at); citations are left alone, so their targets never move.

use std::sync::LazyLock;

use deino_doc::{Doc, Inline};
use regex::Regex;

use super::blocks::for_each_text_block;
use super::scope::Scope;

/// The most words a name in a rename may have.
pub const MAX_NAME_WORDS: usize = 4;

/// Words a description of a part of the notes starts with.
const NOT_A_NAME: [&str; 9] = [
    "the", "a", "an", "this", "that", "these", "those", "all", "every",
];

/// `(pattern, whether the new name comes first)`.
static PATTERNS: LazyLock<Vec<(Regex, bool)>> = LazyLock::new(|| {
    [
        (r"^(?:please\s+)?rename\s+(.+?)\s+(?:to|as)\s+(.+)$", false),
        (r"^(?:please\s+)?replace\s+(.+?)\s+with\s+(.+)$", false),
        (r"^(?:please\s+)?spell\s+(.+?)\s+as\s+(.+)$", false),
        (
            r"^(.+?)\s+is\s+(?:actually\s+|really\s+)?(?:spelled|spelt)\s+(.+)$",
            false,
        ),
        (r"^(.+?)\s*(?:->|→)\s*(.+)$", false),
        (r"^(?:it'?s|it’s|it\s+is)\s+(.+?)\s*,?\s+not\s+(.+)$", true),
    ]
    .into_iter()
    .filter_map(|(pattern, reversed)| {
        Regex::new(&format!("(?i){pattern}"))
            .ok()
            .map(|regex| (regex, reversed))
    })
    .collect()
});

/// Trailing words that only say "everywhere".
static EVERYWHERE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?i)\s+(?:everywhere|throughout|in\s+the\s+notes|in\s+all\s+the\s+notes|please)$")
        .ok()
});

/// A plain rename: every `from` becomes `to`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rename {
    /// The name as the notes have it.
    pub from: String,
    /// The name it becomes.
    pub to: String,
}

impl Rename {
    /// The rename `instruction` asks for, or `None` when it asks for
    /// anything else.
    pub fn parse(instruction: &str) -> Option<Self> {
        let request = strip_everywhere(instruction.trim().trim_end_matches(['.', '!']));
        PATTERNS.iter().find_map(|(pattern, reversed)| {
            let captures = pattern.captures(&request)?;
            let first = name(captures.get(1)?.as_str())?;
            let second = name(captures.get(2)?.as_str())?;
            let (from, to) = if *reversed {
                (second, first)
            } else {
                (first, second)
            };
            (from != to).then_some(Self { from, to })
        })
    }

    /// `doc` with the rename applied inside `scope`, and how many places
    /// changed.
    pub fn apply(&self, doc: &Doc, scope: &Scope) -> (Doc, u32) {
        let mut renamed = doc.clone();
        let mut count = 0;
        for_each_text_block(&mut renamed, &mut |index, content| {
            if scope.contains(index) {
                count += content
                    .iter_mut()
                    .map(|inline| self.rename_inline(inline))
                    .sum::<u32>();
            }
        });
        (renamed.normalized(), count)
    }

    fn rename_inline(&self, inline: &mut Inline) -> u32 {
        let text = match inline {
            Inline::Text { text, .. } => text,
            Inline::Mention { attrs, .. } => &mut attrs.label,
            Inline::HardBreak { .. } | Inline::Citation { .. } => return 0,
        };
        let (replaced, count) = self.replace_in(text);
        *text = replaced;
        count
    }

    /// `text` with every whole-word `from` replaced.
    fn replace_in(&self, text: &str) -> (String, u32) {
        let Ok(pattern) = Regex::new(&format!("(?i){}", regex::escape(&self.from))) else {
            return (text.to_owned(), 0);
        };
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        let mut count = 0;
        for found in pattern.find_iter(text) {
            if !is_whole_word(text, found.start(), found.end()) {
                continue;
            }
            out.push_str(text.get(last..found.start()).unwrap_or_default());
            out.push_str(&self.cased_for(found.as_str()));
            last = found.end();
            count += 1;
        }
        out.push_str(text.get(last..).unwrap_or_default());
        (out, count)
    }

    /// The new name, capitalized when the occurrence is and the user typed
    /// it in lower case.
    fn cased_for(&self, occurrence: &str) -> String {
        let capital = occurrence.chars().next().is_some_and(char::is_uppercase);
        let mut chars = self.to.chars();
        match chars.next() {
            Some(first) if capital && first.is_lowercase() => {
                first.to_uppercase().chain(chars).collect()
            }
            _ => self.to.clone(),
        }
    }
}

fn strip_everywhere(request: &str) -> String {
    let mut request = request.to_owned();
    while let Some(found) = EVERYWHERE.as_ref().and_then(|tail| tail.find(&request)) {
        request.truncate(found.start());
    }
    request
}

/// A name of a rename, without quotes; `None` when it is not one.
fn name(raw: &str) -> Option<String> {
    let name = raw
        .trim()
        .trim_matches(['"', '\'', '“', '”', '‘', '’', '`'])
        .trim();
    let words: Vec<&str> = name.split_whitespace().collect();
    let first = words.first()?.to_lowercase();
    let named = words.len() <= MAX_NAME_WORDS
        && name.chars().any(char::is_alphanumeric)
        && !NOT_A_NAME.contains(&first.as_str());
    named.then(|| name.to_owned())
}

/// Whether `text[start..end]` is not part of a longer word.
fn is_whole_word(text: &str, start: usize, end: usize) -> bool {
    let before = text.get(..start).and_then(|head| head.chars().next_back());
    let after = text.get(end..).and_then(|tail| tail.chars().next());
    let first = text.get(start..end).and_then(|word| word.chars().next());
    let last = text
        .get(start..end)
        .and_then(|word| word.chars().next_back());
    let joins = |edge: Option<char>, neighbour: Option<char>| {
        edge.is_some_and(is_word_char) && neighbour.is_some_and(is_word_char)
    };
    !joins(first, before) && !joins(last, after)
}

fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod rename_tests;
