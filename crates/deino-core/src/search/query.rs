//! The search box's little language (Circleback §9.1).
//!
//! | Typed | Means |
//! |---|---|
//! | `roadmap budget` | both words, anywhere (the last one as a prefix while typing) |
//! | `"launch date"` | the exact phrase |
//! | `@maya`, `@"Maya Chen"` | a filter by name: a person, company, tag, folder or meeting |
//! | `@tag:roadmap`, `@folder:"Q4 planning"` | a filter of one kind |
//! | `acme.com` | meetings with an attendee from that domain |
//!
//! Parsing never fails: anything unrecognised is a word. Mentions and
//! domains are only candidates here; the service resolves them against the
//! database and turns the ones that match nothing back into words.

use super::FilterKind;

/// A search term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    /// A word, matched by its stem (and as a prefix when it is the last word
    /// of an open-ended query).
    Word(String),
    /// An exact phrase.
    Phrase(String),
}

/// An `@` filter by name, not resolved yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    /// The kind written before a colon (`@tag:…`), if any.
    pub kind: Option<FilterKind>,
    /// The name after the `@` (and the kind).
    pub name: String,
}

/// A parsed query.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedQuery {
    /// Words and phrases, in order.
    pub terms: Vec<Term>,
    /// `@` filters, in order.
    pub mentions: Vec<Mention>,
    /// Bare domains such as `acme.com`, lower case.
    pub domains: Vec<String>,
    /// Whether the user is still typing the last word (no trailing space),
    /// so it should match as a prefix.
    pub open_ended: bool,
}

/// One whitespace-separated piece of the input.
#[derive(Debug, PartialEq, Eq)]
enum Token {
    Bare(String),
    Quoted(String),
    Mention {
        head: String,
        quoted: Option<String>,
    },
}

/// Parses what the user typed.
pub fn parse(text: &str) -> ParsedQuery {
    let mut query = ParsedQuery {
        open_ended: !text.is_empty() && !text.ends_with(char::is_whitespace),
        ..ParsedQuery::default()
    };
    for token in tokens(text) {
        match token {
            Token::Quoted(phrase) => push_phrase(&mut query, &phrase),
            Token::Mention { head, quoted } => push_mention(&mut query, &head, quoted),
            Token::Bare(word) => push_bare(&mut query, word),
        }
    }
    query
}

fn push_phrase(query: &mut ParsedQuery, phrase: &str) {
    let phrase = phrase.split_whitespace().collect::<Vec<_>>().join(" ");
    if has_word_chars(&phrase) {
        query.terms.push(Term::Phrase(phrase));
    }
}

fn push_mention(query: &mut ParsedQuery, head: &str, quoted: Option<String>) {
    let head = head.trim_start_matches('@');
    let (kind, rest) = match head.split_once(':') {
        Some((kind, rest)) => match filter_kind(kind) {
            Some(kind) => (Some(kind), rest),
            None => (None, head),
        },
        None => (None, head),
    };
    let name = quoted.unwrap_or_else(|| rest.to_owned());
    let name = name.trim();
    if !name.is_empty() {
        query.mentions.push(Mention {
            kind,
            name: name.to_owned(),
        });
    }
}

fn push_bare(query: &mut ParsedQuery, word: String) {
    if let Some(domain) = domain(&word) {
        query.domains.push(domain);
    } else if has_word_chars(&word) {
        query.terms.push(Term::Word(word));
    }
}

/// The filter kind named before a colon.
fn filter_kind(name: &str) -> Option<FilterKind> {
    Some(match name.to_ascii_lowercase().as_str() {
        "person" | "people" => FilterKind::Person,
        "company" => FilterKind::Company,
        "tag" => FilterKind::Tag,
        "folder" => FilterKind::Folder,
        "meeting" => FilterKind::Meeting,
        _ => return None,
    })
}

/// Splits `text` into tokens, honouring double quotes.
fn tokens(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        let (token, after) = next_token(rest);
        tokens.push(token);
        rest = after.trim_start();
    }
    tokens
}

/// The token at the start of `text` (which starts with a non-space) and
/// what follows it.
fn next_token(text: &str) -> (Token, &str) {
    if let Some(inner) = text.strip_prefix('"') {
        let (quoted, after) = until_quote(inner);
        return (Token::Quoted(quoted.to_owned()), after);
    }
    let end = text
        .find(|c: char| c.is_whitespace() || c == '"')
        .unwrap_or(text.len());
    let (bare, after) = text.split_at(end);
    let mention_with_quote = bare.starts_with('@') && (bare == "@" || bare.ends_with(':'));
    match after.strip_prefix('"') {
        Some(inner) if mention_with_quote => {
            let (quoted, after) = until_quote(inner);
            let token = Token::Mention {
                head: bare.to_owned(),
                quoted: Some(quoted.to_owned()),
            };
            (token, after)
        }
        _ if bare.starts_with('@') => {
            let token = Token::Mention {
                head: bare.to_owned(),
                quoted: None,
            };
            (token, after)
        }
        _ => (Token::Bare(bare.to_owned()), after),
    }
}

/// The text up to the closing quote (or the end) and what follows it.
fn until_quote(text: &str) -> (&str, &str) {
    match text.split_once('"') {
        Some((quoted, after)) => (quoted, after),
        None => (text, ""),
    }
}

/// Whether `text` has a letter or digit, so it can match an indexed token.
fn has_word_chars(text: &str) -> bool {
    text.chars().any(char::is_alphanumeric)
}

/// `word` as a lower-case domain, if it looks like one: two or more labels
/// of letters, digits and inner hyphens, ending in a top-level domain of at
/// least two letters (so `v1.4` and `3.5` stay words). Trailing punctuation
/// is ignored.
fn domain(word: &str) -> Option<String> {
    let word = word
        .trim_end_matches([',', ';', ':', '!', '?', ')', '.'])
        .to_ascii_lowercase();
    let labels: Vec<&str> = word.split('.').collect();
    let (tld, _) = labels.split_last()?;
    let valid = labels.len() >= 2
        && labels.iter().all(|label| is_label(label))
        && tld.len() >= 2
        && tld.chars().all(|c| c.is_ascii_lowercase());
    valid.then_some(word)
}

fn is_label(label: &str) -> bool {
    !label.is_empty()
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
