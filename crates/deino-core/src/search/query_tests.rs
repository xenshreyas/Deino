use super::*;

fn word(text: &str) -> Term {
    Term::Word(text.to_owned())
}

fn phrase(text: &str) -> Term {
    Term::Phrase(text.to_owned())
}

fn mention(kind: Option<FilterKind>, name: &str) -> Mention {
    Mention {
        kind,
        name: name.to_owned(),
    }
}

/// One row of the table: input, then what each part of the result must be.
struct Case {
    input: &'static str,
    terms: Vec<Term>,
    mentions: Vec<Mention>,
    domains: Vec<&'static str>,
}

fn case(input: &'static str, terms: Vec<Term>) -> Case {
    Case {
        input,
        terms,
        mentions: Vec::new(),
        domains: Vec::new(),
    }
}

fn cases() -> Vec<Case> {
    let mut cases = word_cases();
    cases.extend(filter_cases());
    cases
}

/// Words, phrases and the things that only look like domains.
fn word_cases() -> Vec<Case> {
    vec![
        case("", vec![]),
        case("   ", vec![]),
        case("roadmap budget", vec![word("roadmap"), word("budget")]),
        case(
            "\"launch date\" slip",
            vec![phrase("launch date"), word("slip")],
        ),
        case("\"  spaced   out \"", vec![phrase("spaced out")]),
        case("\"unterminated phrase", vec![phrase("unterminated phrase")]),
        case("\"\" - & --", vec![]),
        case(
            "v1.4 3.5 node",
            vec![word("v1.4"), word("3.5"), word("node")],
        ),
        case("maya@deino.com", vec![word("maya@deino.com")]),
        Case {
            domains: vec!["acme.com"],
            ..case("demo Acme.com,", vec![word("demo")])
        },
        Case {
            domains: vec!["eu.acme-corp.co.uk"],
            ..case("eu.acme-corp.co.uk", vec![])
        },
        case(
            "-acme.com bad-.com",
            vec![word("-acme.com"), word("bad-.com")],
        ),
        // An `@` inside a word is part of the word, not a mention.
        case("budget@maya", vec![word("budget@maya")]),
    ]
}

/// Mentions, with and without a kind and quotes.
fn filter_cases() -> Vec<Case> {
    vec![
        Case {
            mentions: vec![mention(None, "maya")],
            ..case("@maya sync", vec![word("sync")])
        },
        Case {
            mentions: vec![mention(None, "Maya Chen")],
            ..case("@\"Maya Chen\" roadmap", vec![word("roadmap")])
        },
        Case {
            mentions: vec![
                mention(Some(FilterKind::Tag), "roadmap"),
                mention(Some(FilterKind::Folder), "Q4 planning"),
                mention(Some(FilterKind::Person), "maya"),
                mention(Some(FilterKind::Company), "acme"),
                mention(Some(FilterKind::Meeting), "standup"),
            ],
            ..case(
                "@tag:roadmap @folder:\"Q4 planning\" @people:maya @company:acme @meeting:standup",
                vec![],
            )
        },
        Case {
            mentions: vec![mention(None, "time:10am")],
            ..case("@time:10am", vec![])
        },
        case("@ @\"\" @tag:", vec![]),
    ]
}

#[test]
fn every_form_of_the_query_language_parses_as_documented() {
    for case in cases() {
        let parsed = parse(case.input);

        assert_eq!(parsed.terms, case.terms, "terms of {:?}", case.input);
        assert_eq!(
            parsed.mentions, case.mentions,
            "mentions of {:?}",
            case.input
        );
        let domains: Vec<&str> = parsed.domains.iter().map(String::as_str).collect();
        assert_eq!(domains, case.domains, "domains of {:?}", case.input);
    }
}

#[test]
fn a_query_without_a_trailing_space_is_still_being_typed() {
    assert!(parse("road").open_ended);
    assert!(!parse("road ").open_ended);
    assert!(!parse("").open_ended);
}
