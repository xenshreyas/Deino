//! Compact Markdown for every output: what the terminal prints without
//! `--json` and what the MCP tools return as text. It reads well raw, keeps
//! IDs in backticks so they can be copied into the next call, and spends as
//! few tokens as it can.

mod directory;
mod items;
mod meetings;
mod search;
mod transcript;

use chrono::{DateTime, FixedOffset, NaiveDate};

/// A moment, such as `Thu 8 Oct 2026, 15:31`.
pub(crate) fn when(at: &DateTime<FixedOffset>) -> String {
    at.format("%a %-d %b %Y, %H:%M").to_string()
}

/// A day, such as `Fri 9 Oct 2026`.
pub(crate) fn day(date: NaiveDate) -> String {
    date.format("%a %-d %b %Y").to_string()
}

/// A length, such as `7 min` or `1 h 23 min`.
pub(crate) fn length(seconds: u32) -> String {
    let minutes = (seconds + 30) / 60;
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

/// `count` with its noun, such as `1 meeting` or `3 meetings`.
pub(crate) fn counted(count: u32, one: &str, many: &str) -> String {
    if count == 1 {
        format!("1 {one}")
    } else {
        format!("{count} {many}")
    }
}

/// `items` joined with `, `, or `None` when there are none.
pub(crate) fn joined(items: &[String]) -> Option<String> {
    (!items.is_empty()).then(|| items.join(", "))
}

/// Appends a line to `out`.
pub(crate) fn line(out: &mut String, text: impl std::fmt::Display) {
    out.push_str(&text.to_string());
    out.push('\n');
}

/// Markdown written by someone else, nested under a `##` section: its
/// headings move down two levels so they stay inside it.
pub(crate) fn nested(markdown: &str) -> String {
    markdown
        .lines()
        .map(|text| {
            if text.starts_with('#') {
                format!("##{text}")
            } else {
                text.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_round_to_minutes() {
        assert_eq!(length(434), "7 min");
        assert_eq!(length(3_600), "1 h");
        assert_eq!(length(4_980), "1 h 23 min");
    }

    #[test]
    fn nested_headings_move_down() {
        assert_eq!(nested("## Decisions\n- ship"), "#### Decisions\n- ship");
    }

    #[test]
    fn moments_and_days_read_naturally() {
        let at = DateTime::parse_from_rfc3339("2026-10-08T15:31:04+00:00").unwrap();

        assert_eq!(when(&at), "Thu 8 Oct 2026, 15:31");
        assert_eq!(day(at.date_naive()), "Thu 8 Oct 2026");
    }
}
