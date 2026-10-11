//! Turning parsed terms into an FTS5 `MATCH` expression.
//!
//! Every term becomes a double-quoted FTS5 string, so nothing the user types
//! is read as query syntax (`AND`, `NEAR`, `*`, `:` and parentheses are all
//! literal inside quotes), and terms are joined by FTS5's implicit AND. The
//! tokenizer splits each string the way it split the documents, so `v1.4`
//! matches the tokens `v1` `4` next to each other. While the user is still
//! typing the last word, it matches as a prefix (`"roadm"*`), served by the
//! table's 2- and 3-character prefix indexes.

use super::query::Term;

/// The `MATCH` expression for `terms`, or `None` when there is nothing to
/// match. `open_ended` makes the last word a prefix.
pub fn match_expression(terms: &[Term], open_ended: bool) -> Option<String> {
    let last = terms.len().checked_sub(1)?;
    let parts: Vec<String> = terms
        .iter()
        .enumerate()
        .map(|(index, term)| match term {
            Term::Word(word) if open_ended && index == last => format!("{}*", quoted(word)),
            Term::Word(text) | Term::Phrase(text) => quoted(text),
        })
        .collect();
    Some(parts.join(" "))
}

/// The `MATCH` expression for any of `terms` (FTS5 `OR`), for questions in
/// natural language, where requiring every word would match nothing; BM25
/// still ranks documents with more (and rarer) words first.
pub fn any_expression(terms: &[Term]) -> Option<String> {
    let parts: Vec<String> = terms
        .iter()
        .map(|term| match term {
            Term::Word(text) | Term::Phrase(text) => quoted(text),
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" OR "))
}

/// `text` as an FTS5 string: double quotes doubled, wrapped in quotes.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// The words of `terms` in lower case, for highlighting text that FTS5 did
/// not mark (semantic hits).
pub fn words(terms: &[Term]) -> Vec<String> {
    terms
        .iter()
        .flat_map(|term| match term {
            Term::Word(text) | Term::Phrase(text) => text.split(|c: char| !c.is_alphanumeric()),
        })
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str) -> Term {
        Term::Word(text.to_owned())
    }

    #[test]
    fn terms_are_quoted_and_joined_by_implicit_and() {
        let terms = [word("roadmap"), Term::Phrase("launch date".into())];

        assert_eq!(
            match_expression(&terms, false).unwrap(),
            "\"roadmap\" \"launch date\""
        );
    }

    #[test]
    fn only_the_last_word_of_an_open_ended_query_is_a_prefix() {
        let terms = [word("budget"), word("roadm")];

        assert_eq!(
            match_expression(&terms, true).unwrap(),
            "\"budget\" \"roadm\"*"
        );
        let phrase_last = [Term::Phrase("launch date".into())];
        assert_eq!(
            match_expression(&phrase_last, true).unwrap(),
            "\"launch date\""
        );
    }

    #[test]
    fn query_syntax_typed_by_the_user_stays_literal() {
        let terms = [word("NEAR(a"), word("say\"hi\""), word("OR")];

        assert_eq!(
            match_expression(&terms, false).unwrap(),
            "\"NEAR(a\" \"say\"\"hi\"\"\" \"OR\""
        );
    }

    #[test]
    fn no_terms_match_nothing() {
        assert_eq!(match_expression(&[], true), None);
        assert_eq!(any_expression(&[]), None);
    }

    #[test]
    fn a_question_matches_any_of_its_terms() {
        let terms = [word("when"), Term::Phrase("beta date".into())];

        assert_eq!(any_expression(&terms).unwrap(), "\"when\" OR \"beta date\"");
    }

    #[test]
    fn words_split_phrases_and_punctuation() {
        let terms = [word("v1.4"), Term::Phrase("Launch Date".into())];

        assert_eq!(words(&terms), ["v1", "4", "launch", "date"]);
    }
}
