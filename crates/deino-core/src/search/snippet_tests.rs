use super::*;

fn highlighted(snippet: &Snippet) -> Vec<String> {
    let units: Vec<u16> = snippet.text.encode_utf16().collect();
    snippet
        .highlights
        .iter()
        .map(|range| String::from_utf16(&units[range.start as usize..range.end as usize]).unwrap())
        .collect()
}

#[test]
fn fts_marks_become_highlight_ranges_on_clean_text() {
    let snippet = from_marked("…we \u{2}ship\u{3} the \u{2}roadmap\u{3} today");

    assert_eq!(snippet.text, "…we ship the roadmap today");
    assert_eq!(highlighted(&snippet), ["ship", "roadmap"]);
}

#[test]
fn ranges_count_utf16_code_units_like_javascript() {
    // "é" is one code unit, "📅" two; JavaScript slices by code units.
    let snippet = from_marked("é📅 \u{2}café\u{3}");

    assert_eq!(snippet.highlights, [Highlight { start: 4, end: 8 }]);
    assert_eq!(highlighted(&snippet), ["café"]);
}

#[test]
fn unbalanced_marks_close_at_the_end_and_stray_closes_are_dropped() {
    let snippet = from_marked("a\u{3} \u{2}\u{3}b \u{2}open");

    assert_eq!(snippet.text, "a b open");
    assert_eq!(highlighted(&snippet), ["open"]);
}

#[test]
fn text_without_marks_has_no_highlights() {
    assert_eq!(from_marked("plain"), Snippet::plain("plain"));
}

#[test]
fn an_excerpt_highlights_query_words_and_their_variants() {
    let words = vec!["roadmap".to_owned(), "ship".to_owned()];

    let snippet = excerpt("We shipped the Roadmap; roadmaps ship.", &words);

    assert_eq!(snippet.text, "We shipped the Roadmap; roadmaps ship.");
    assert_eq!(
        highlighted(&snippet),
        ["shipped", "Roadmap", "roadmaps", "ship"]
    );
}

#[test]
fn a_long_excerpt_starts_near_the_first_match_and_is_cut_with_ellipses() {
    let text = format!(
        "{} the budget slipped {}",
        "early ".repeat(40),
        "late ".repeat(80)
    );

    let snippet = excerpt(&text, &["budget".to_owned()]);

    assert!(snippet.text.starts_with(ELLIPSIS), "{}", snippet.text);
    assert!(snippet.text.ends_with(ELLIPSIS));
    assert!(snippet.text.chars().count() <= MAX_EXCERPT_CHARS + 2);
    assert_eq!(highlighted(&snippet), ["budget"]);
}

#[test]
fn the_best_line_is_the_one_with_the_most_query_words() {
    let text = "Maya: hello there\nTom: the beta ships in November\nMaya: beta";
    let words = vec!["beta".to_owned(), "november".to_owned()];

    assert_eq!(best_line(text, &words), "Tom: the beta ships in November");
    assert_eq!(best_line(text, &["zzz".to_owned()]), "Maya: hello there");
    assert_eq!(best_line("", &words), "");
}

#[test]
fn an_excerpt_without_a_match_is_the_start_of_the_text() {
    let snippet = excerpt("Nothing in common here.", &["roadmap".to_owned()]);

    assert_eq!(snippet, Snippet::plain("Nothing in common here."));
}
