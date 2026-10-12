//! [`Lexicon`]: the vocabulary as matching rules.
//!
//! - **Phrases** are matched exactly, ignoring case, as whole words: a
//!   replacement's variants become its text (keeping a capital first letter);
//!   a term's variants and any casing of the term become the term as spelled;
//!   a known name is kept as it is, so nothing fuzzy rewrites it.
//! - **Names** are matched fuzzily (Jaro-Winkler): people, companies, calendar
//!   invitees and terms that start with a capital letter. People also count
//!   by their first name.
//!
//! A user's entry wins over a name with the same words, and an earlier entry
//! over a later one.

use std::collections::HashMap;

use deino_domain::VocabKind;

use super::VocabTerm;
use super::tokens::{fold, letters};

/// Where a known name comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameKind {
    /// Someone the user meets (a person or a calendar invitee).
    Person,
    /// A company.
    Company,
}

/// A name the transcript may contain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownName {
    /// The name as spelled.
    pub text: String,
    /// Whose name it is.
    pub kind: NameKind,
}

/// What an exactly matched phrase becomes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Stays as heard: a known name, protected from fuzzy matching.
    Keep,
    /// Spelled exactly as given (a term).
    Spell(String),
    /// Replaced by this text, capitalised like the words it replaces.
    Replace(String),
}

/// A phrase matched exactly, word for word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Phrase {
    /// Its words, folded.
    pub words: Vec<String>,
    /// What it becomes.
    pub outcome: Outcome,
}

/// A name matched fuzzily.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FuzzyName {
    /// The name as spelled.
    pub text: String,
    /// Its letters, folded, as compared.
    pub key: String,
    /// How many words it has.
    pub words: usize,
}

/// The vocabulary as matching rules.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lexicon {
    /// Phrases by their first folded word, longest first.
    phrases: HashMap<String, Vec<Phrase>>,
    /// Fuzzy names by the first letter of their key.
    names: HashMap<char, Vec<FuzzyName>>,
}

impl Lexicon {
    /// The rules of the user's `terms` and the `names` the meeting may mention.
    pub fn new(terms: &[VocabTerm], names: &[KnownName]) -> Self {
        let mut lexicon = Self::default();
        for term in terms {
            lexicon.add_term(term);
        }
        for name in names {
            lexicon.add_name(name);
        }
        for bucket in lexicon.phrases.values_mut() {
            bucket.sort_by_key(|phrase| std::cmp::Reverse(phrase.words.len()));
        }
        lexicon
    }

    /// True when nothing would ever be corrected.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
            && self
                .phrases
                .values()
                .flatten()
                .all(|phrase| phrase.outcome == Outcome::Keep)
    }

    /// The phrases starting with the folded word `first`, longest first.
    pub(super) fn phrases(&self, first: &str) -> &[Phrase] {
        self.phrases.get(first).map_or(&[], Vec::as_slice)
    }

    /// The fuzzy names whose key starts with `letter`.
    pub(super) fn names(&self, letter: char) -> &[FuzzyName] {
        self.names.get(&letter).map_or(&[], Vec::as_slice)
    }

    fn add_term(&mut self, term: &VocabTerm) {
        let outcome = match term.kind {
            VocabKind::Term => Outcome::Spell(term.text.clone()),
            VocabKind::Replacement => Outcome::Replace(term.text.clone()),
        };
        for variant in &term.sounds_like {
            self.add_phrase(variant, outcome.clone());
        }
        if term.kind == VocabKind::Term {
            self.add_phrase(&term.text, outcome);
            if starts_upper(&term.text) {
                self.add_fuzzy(&term.text);
            }
        }
    }

    fn add_name(&mut self, name: &KnownName) {
        self.add_phrase(&name.text, Outcome::Keep);
        self.add_fuzzy(&name.text);
        let mut words = name.text.split_whitespace();
        if let (NameKind::Person, Some(first), Some(_)) = (name.kind, words.next(), words.next()) {
            self.add_phrase(first, Outcome::Keep);
            self.add_fuzzy(first);
        }
    }

    /// Adds a phrase unless one with the same words is already there.
    fn add_phrase(&mut self, text: &str, outcome: Outcome) {
        let words: Vec<String> = text.split_whitespace().map(fold).collect();
        let Some(first) = words.first().cloned() else {
            return;
        };
        let bucket = self.phrases.entry(first).or_default();
        if !bucket.iter().any(|phrase| phrase.words == words) {
            bucket.push(Phrase { words, outcome });
        }
    }

    /// Adds a fuzzy name unless one with the same letters is already there.
    fn add_fuzzy(&mut self, text: &str) {
        let key = letters(text);
        let Some(letter) = key.chars().next() else {
            return;
        };
        let bucket = self.names.entry(letter).or_default();
        if !bucket.iter().any(|name| name.key == key) {
            bucket.push(FuzzyName {
                text: text.split_whitespace().collect::<Vec<_>>().join(" "),
                words: text.split_whitespace().count(),
                key,
            });
        }
    }
}

/// True when `text` starts with a capital letter (a proper noun).
fn starts_upper(text: &str) -> bool {
    text.chars().next().is_some_and(char::is_uppercase)
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use deino_domain::{FakeClock, VocabTermId};

    use super::*;

    fn term(kind: VocabKind, text: &str, sounds_like: &[&str]) -> VocabTerm {
        let clock = FakeClock::new(DateTime::UNIX_EPOCH);
        VocabTerm {
            id: VocabTermId::generate(&clock),
            kind,
            text: text.into(),
            sounds_like: sounds_like.iter().map(|&s| s.to_owned()).collect(),
            created_at: DateTime::UNIX_EPOCH,
            updated_at: DateTime::UNIX_EPOCH,
        }
    }

    fn person(text: &str) -> KnownName {
        KnownName {
            text: text.into(),
            kind: NameKind::Person,
        }
    }

    #[test]
    fn user_entries_win_over_names_with_the_same_words() {
        let lexicon = Lexicon::new(
            &[term(VocabKind::Term, "Shreyas", &["Sreyas"])],
            &[person("Sreyas Rao"), person("Shreyas Patel")],
        );

        let sreyas = lexicon.phrases("sreyas");
        assert_eq!(sreyas[0].words, ["sreyas", "rao"]);
        assert_eq!(sreyas[1].outcome, Outcome::Spell("Shreyas".into()));
        assert_eq!(lexicon.phrases("shreyas").len(), 2);
    }

    #[test]
    fn people_count_by_first_name_and_lowercase_terms_are_not_fuzzy() {
        let lexicon = Lexicon::new(
            &[term(VocabKind::Term, "kubectl", &[])],
            &[person("Shreyas Patel")],
        );

        let keys: Vec<&str> = lexicon.names('s').iter().map(|n| n.key.as_str()).collect();
        assert_eq!(keys, ["shreyaspatel", "shreyas"]);
        assert!(lexicon.names('k').is_empty());
    }

    #[test]
    fn a_lexicon_of_kept_names_alone_still_corrects_fuzzily() {
        assert!(Lexicon::default().is_empty());
        assert!(!Lexicon::new(&[], &[person("Shreyas")]).is_empty());
        let replacement = term(VocabKind::Replacement, "going to", &["gonna"]);
        assert!(!Lexicon::new(&[replacement], &[]).is_empty());
    }
}
