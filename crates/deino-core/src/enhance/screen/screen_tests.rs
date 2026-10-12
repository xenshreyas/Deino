//! The screen's lines: flattened, de-duplicated, capped, and placed among
//! the transcript lines by time.

use super::*;
use crate::enhance::test_support::line;

fn shown(seconds: u32, text: &str, description: Option<&str>) -> ScreenText {
    ScreenText {
        at: Millis(seconds * 1_000),
        text: text.to_owned(),
        description: description.map(str::to_owned),
    }
}

#[test]
fn a_snapshot_is_one_line_its_description_first() {
    let lines = screen_lines(&[shown(
        724,
        "Q3 Review\n  Revenue up 40% \n",
        Some("A title slide."),
    )]);

    assert_eq!(
        lines.iter().map(ScreenLine::render).collect::<Vec<_>>(),
        ["[screen 12:04] A title slide. · Q3 Review · Revenue up 40%"]
    );
}

#[test]
fn a_repeated_slide_and_an_empty_one_are_dropped() {
    let lines = screen_lines(&[
        shown(10, "Roadmap", None),
        shown(20, "Roadmap", None),
        shown(30, "  ", None),
        shown(40, "Hiring plan", None),
    ]);

    let times: Vec<Millis> = lines.iter().map(|line| line.at).collect();
    assert_eq!(times, [Millis(10_000), Millis(40_000)]);
}

#[test]
fn the_screen_never_crowds_out_the_transcript() {
    let long = "x".repeat(MAX_SCREEN_LINE_CHARS * 2);
    let texts: Vec<ScreenText> = (0..20)
        .map(|n| shown(n * 10, &format!("{n}{long}"), None))
        .collect();

    let lines = screen_lines(&texts);

    assert!(lines.iter().all(|line| line.text.chars().count() <= MAX_SCREEN_LINE_CHARS));
    let total: usize = lines.iter().map(|line| line.text.chars().count()).sum();
    assert!(total <= MAX_SCREEN_CHARS, "{total}");
}

#[test]
fn screen_lines_sit_where_they_were_shown() {
    let transcript = [
        line(1, 0, "Maya", "let me share my screen"),
        line(2, 30, "Maya", "revenue grew a lot"),
    ];
    let screen = screen_lines(&[shown(12, "Revenue up 40%", None), shown(90, "Thanks", None)]);

    assert_eq!(
        render_with_screen(&transcript, &screen),
        "[s1 00:00] Maya: let me share my screen\n\
         [screen 00:12] Revenue up 40%\n\
         [s2 00:30] Maya: revenue grew a lot\n\
         [screen 01:30] Thanks"
    );
}

#[test]
fn a_part_takes_the_screen_lines_of_its_span() {
    let first = [line(1, 0, "Maya", "a"), line(2, 600, "Maya", "b")];
    let second = [line(3, 1_200, "Maya", "c")];
    let screen = screen_lines(&[
        shown(5, "one", None),
        shown(1_000, "gap", None),
        shown(1_300, "two", None),
    ]);

    let texts = |lines: Vec<ScreenLine>| lines.into_iter().map(|l| l.text).collect::<Vec<_>>();
    assert_eq!(texts(within(&screen, &first, (true, false))), ["one"]);
    assert_eq!(texts(within(&screen, &second, (false, true))), ["two"]);
}
