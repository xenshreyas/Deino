use chrono::DateTime;
use deino_domain::{FakeClock, Millis, VocabKind, VocabTermId};
use proptest::prelude::*;

use super::*;
use crate::vocabulary::{KnownName, NameKind, VocabTerm};

fn term(kind: VocabKind, text: &str, sounds_like: &[&str]) -> VocabTerm {
    VocabTerm {
        id: VocabTermId::generate(&FakeClock::new(DateTime::UNIX_EPOCH)),
        kind,
        text: text.into(),
        sounds_like: sounds_like.iter().map(|&s| s.to_owned()).collect(),
        created_at: DateTime::UNIX_EPOCH,
        updated_at: DateTime::UNIX_EPOCH,
    }
}

fn person(name: &str) -> KnownName {
    KnownName {
        text: name.into(),
        kind: NameKind::Person,
    }
}

fn company(name: &str) -> KnownName {
    KnownName {
        text: name.into(),
        kind: NameKind::Company,
    }
}

fn corrector(terms: &[VocabTerm], names: &[KnownName]) -> Corrector {
    Corrector::new(Lexicon::new(terms, names))
}

/// The user's own vocabulary: their name with how it is misheard, a product
/// with its casing, and a plain replacement rule.
fn users_vocabulary() -> Corrector {
    corrector(
        &[
            term(VocabKind::Term, "Shreyas", &["Sreyas", "Shrey as"]),
            term(VocabKind::Term, "PagerDuty", &["pager duty"]),
            term(VocabKind::Replacement, "going to", &["gonna"]),
        ],
        &[],
    )
}

fn corrected(corrector: &Corrector, text: &str) -> String {
    corrector.text(text).unwrap_or_else(|| text.to_owned())
}

#[test]
fn shreyas_misheard_as_sreyas_is_corrected() {
    let vocabulary = users_vocabulary();

    assert_eq!(
        corrected(&vocabulary, "Thanks Sreyas, and sreyas's plan."),
        "Thanks Shreyas, and Shreyas's plan."
    );
    assert_eq!(corrected(&vocabulary, "(Shrey as)"), "(Shreyas)");
}

#[test]
fn a_misheard_attendee_is_corrected_from_the_names_alone() {
    let names = corrector(&[], &[person("Shreyas Patel"), company("Fernhollow Labs")]);

    assert_eq!(
        corrected(&names, "Sreyas Patel joined from Fernholow Labs."),
        "Shreyas Patel joined from Fernhollow Labs."
    );
    assert_eq!(
        corrected(&names, "Over to you, Sreyas."),
        "Over to you, Shreyas."
    );
}

#[test]
fn corrections_never_land_inside_other_words() {
    let vocabulary = users_vocabulary();

    for text in [
        "Sreyasan",
        "unsreyas",
        "gonnas",
        "pager dutyless",
        "Shreyasa",
    ] {
        assert_eq!(vocabulary.text(text), None, "{text}");
    }
}

#[test]
fn terms_keep_their_spelling_and_replacements_their_capital() {
    let vocabulary = users_vocabulary();

    assert_eq!(
        corrected(&vocabulary, "Gonna page pagerduty, then pager duty."),
        "Going to page PagerDuty, then PagerDuty."
    );
    assert_eq!(vocabulary.text("PagerDuty is fine"), None);
}

#[test]
fn common_words_and_other_known_names_are_left_alone() {
    let names = corrector(
        &[],
        &[
            person("Hunter Smith"),
            person("Christian Bale"),
            person("Sam Lee"),
            person("Kim Smith"),
            person("Tim Smith"),
        ],
    );

    let text = "They hunted the same Christians; Tim Smith and Hunted agreed.";
    assert_eq!(names.text(text), None);
}

#[test]
fn a_plural_or_a_name_with_a_dropped_letter_is_not_a_misheard_name() {
    let names = corrector(&[], &[person("Shreyas Patel"), company("General Motors")]);

    assert_eq!(names.text("The Shreyas and General Motor teams"), None);
}

#[test]
fn replaced_words_keep_their_punctuation_and_share_their_time() {
    let vocabulary = users_vocabulary();
    let words = [
        ("Gonna", 100, 300),
        ("ask", 300, 500),
        ("Sreyas.", 500, 900),
    ]
    .map(|(text, start, end)| WordTiming {
        text: text.into(),
        start: Millis(start),
        end: Millis(end),
    });

    let segment = vocabulary.segment("Gonna ask Sreyas.", &words).unwrap();

    assert_eq!(segment.text, "Going to ask Shreyas.");
    let timed: Vec<(&str, u32, u32)> = segment
        .words
        .iter()
        .map(|w| (w.text.as_str(), w.start.0, w.end.0))
        .collect();
    assert_eq!(
        timed,
        [
            ("Going", 100, 200),
            ("to", 200, 300),
            ("ask", 300, 500),
            ("Shreyas.", 500, 900)
        ]
    );
}

#[test]
fn words_that_do_not_split_one_to_one_are_kept() {
    let vocabulary = users_vocabulary();
    let words = [WordTiming {
        text: "Sreyas said".into(),
        start: Millis(0),
        end: Millis(400),
    }];

    let segment = vocabulary.segment("Sreyas said", &words).unwrap();

    assert_eq!(segment.text, "Shreyas said");
    assert_eq!(segment.words, words);
}

#[test]
fn an_empty_vocabulary_corrects_nothing() {
    let empty = Corrector::default();

    assert!(empty.is_empty());
    assert_eq!(empty.text("Sreyas gonna"), None);
}

/// A word of the generated texts: plain, the misheard name standing alone
/// (with punctuation around it), or the misheard name inside a longer word.
#[derive(Clone, Debug)]
enum Piece {
    Plain(String),
    Name {
        casing: usize,
        before: &'static str,
        after: &'static str,
    },
    Inside {
        before: String,
        after: String,
    },
}

const CASINGS: [&str; 3] = ["Sreyas", "sreyas", "SREYAS"];

fn piece() -> impl Strategy<Value = Piece> {
    let affix = prop::sample::select(vec!["", "(", "\"", "", ""]);
    let suffix = prop::sample::select(vec!["", ",", ".", "'s", "!)", ""]);
    prop_oneof![
        "[b-fk-p]{1,8}".prop_map(Piece::Plain),
        (0..CASINGS.len(), affix, suffix).prop_map(|(casing, before, after)| Piece::Name {
            casing,
            before,
            after
        }),
        ("[b-f]{0,3}", "[b-f]{0,3}")
            .prop_filter("a strictly longer word", |(b, a)| !(b.is_empty()
                && a.is_empty()))
            .prop_map(|(before, after)| Piece::Inside { before, after }),
    ]
}

impl Piece {
    /// The piece as heard, and as it must read after correction.
    fn render(&self) -> (String, String) {
        match self {
            Self::Plain(word) => (word.clone(), word.clone()),
            Self::Name {
                casing,
                before,
                after,
            } => (
                format!(
                    "{before}{}{after}",
                    CASINGS.get(*casing).copied().unwrap_or("Sreyas")
                ),
                format!("{before}Shreyas{after}"),
            ),
            Self::Inside { before, after } => {
                let word = format!("{before}sreyas{after}");
                (word.clone(), word)
            }
        }
    }
}

proptest! {
    /// Only standalone mishearings change, whole and nothing else: plain
    /// words, words that merely contain the name, punctuation and every
    /// separator come through byte for byte.
    #[test]
    fn corrections_replace_whole_words_only(
        pieces in prop::collection::vec(piece(), 0..24),
        separators in prop::collection::vec(prop::sample::select(vec![" ", "  ", "\n", "\t "]), 24),
    ) {
        let vocabulary = users_vocabulary();
        let (mut heard, mut expected) = (String::new(), String::new());
        for (piece, separator) in pieces.iter().zip(&separators) {
            let (h, e) = piece.render();
            heard.push_str(&h);
            heard.push_str(separator);
            expected.push_str(&e);
            expected.push_str(separator);
        }

        prop_assert_eq!(corrected(&vocabulary, &heard), expected);
    }

    /// Correcting a corrected text changes nothing more.
    #[test]
    fn correction_is_idempotent(text in "[a-zA-Z ,.']{0,80}") {
        let vocabulary = corrector(
            &[term(VocabKind::Term, "Shreyas", &["Sreyas"])],
            &[person("Shreyas Patel"), company("Fernhollow Labs")],
        );
        let once = corrected(&vocabulary, &text);

        prop_assert_eq!(corrected(&vocabulary, &once), once);
    }
}
