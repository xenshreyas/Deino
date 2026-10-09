//! The full account export's tables (EXP-02): meetings, action items and
//! people, as RFC 4180 CSV that Numbers, Excel and Sheets open as is.
//!
//! Cells that a spreadsheet would run as a formula (`=`, `+`, `-`, `@`) are
//! prefixed with `'`, so a meeting title can never become one.

use std::collections::BTreeMap;

use crate::{MeetingExport, labels};

/// Characters that make a spreadsheet read a cell as a formula.
const FORMULA_STARTS: &[char] = &['=', '+', '-', '@', '\t', '\r'];

/// One row per meeting, newest first as given.
pub fn meetings(exports: &[MeetingExport]) -> String {
    let header = [
        "id",
        "title",
        "date",
        "duration_minutes",
        "attendees",
        "tags",
        "folders",
        "link",
    ];
    let rows = exports.iter().map(|export| {
        vec![
            export.id.to_string(),
            export.title.clone(),
            labels::iso_local(&export.started_at),
            export
                .duration
                .map(|length| (length.get() / 60).to_string())
                .unwrap_or_default(),
            export
                .attendees
                .iter()
                .map(|person| person.name.as_str())
                .collect::<Vec<_>>()
                .join("; "),
            export.tags.join("; "),
            export.folders.join("; "),
            export.link(),
        ]
    });
    table(&header, rows)
}

/// One row per action item, grouped by meeting.
pub fn action_items(exports: &[MeetingExport]) -> String {
    let header = [
        "meeting_id",
        "meeting_title",
        "title",
        "assignee",
        "due",
        "status",
        "description",
    ];
    let rows = exports.iter().flat_map(|export| {
        export.action_items.iter().map(|item| {
            vec![
                export.id.to_string(),
                export.title.clone(),
                item.title.clone(),
                item.assignee.clone().unwrap_or_default(),
                item.due.map(|date| date.to_string()).unwrap_or_default(),
                if item.done { "done" } else { "open" }.to_owned(),
                item.description.clone(),
            ]
        })
    });
    table(&header, rows)
}

/// One row per person met, by name: their email, how many of the exported
/// meetings they attended and the date of the latest.
pub fn people(exports: &[MeetingExport]) -> String {
    let mut people: BTreeMap<String, Tally> = BTreeMap::new();
    for export in exports {
        let date = export.date();
        for person in &export.attendees {
            let key = person
                .email
                .clone()
                .unwrap_or_else(|| person.name.to_lowercase());
            let tally = people.entry(key).or_insert_with(|| Tally {
                name: person.name.clone(),
                email: person.email.clone().unwrap_or_default(),
                meetings: 0,
                last_met: date,
            });
            tally.meetings += 1;
            tally.last_met = tally.last_met.max(date);
        }
    }
    let mut tallies: Vec<Tally> = people.into_values().collect();
    tallies.sort_by_key(|tally| tally.name.to_lowercase());
    let rows = tallies.into_iter().map(|tally| {
        vec![
            tally.name,
            tally.email,
            tally.meetings.to_string(),
            tally.last_met.to_string(),
        ]
    });
    table(&["name", "email", "meetings", "last_met"], rows)
}

/// A person's attendance across the exported meetings.
struct Tally {
    name: String,
    email: String,
    meetings: u32,
    last_met: chrono::NaiveDate,
}

fn table(header: &[&str], rows: impl Iterator<Item = Vec<String>>) -> String {
    let mut out = header.join(",");
    out.push_str("\r\n");
    for row in rows {
        out.push_str(
            &row.iter()
                .map(|cell| cell_of(cell))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    out
}

/// One cell: formula starts neutralised, quoted when it holds a comma, a
/// quote or a line break.
fn cell_of(value: &str) -> String {
    let value = if value.starts_with(FORMULA_STARTS) {
        format!("'{value}")
    } else {
        value.to_owned()
    };
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture_exports;

    #[test]
    fn the_fixture_account_exports_three_tables() {
        let exports = fixture_exports();
        insta::assert_snapshot!("csv_meetings", meetings(&exports));
        insta::assert_snapshot!("csv_action_items", action_items(&exports));
        insta::assert_snapshot!("csv_people", people(&exports));
    }

    #[test]
    fn cells_are_quoted_and_never_formulas() {
        assert_eq!(cell_of("plain"), "plain");
        assert_eq!(cell_of("a, \"b\"\nc"), "\"a, \"\"b\"\"\nc\"");
        assert_eq!(cell_of("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(cell_of("-1 day"), "'-1 day");
    }
}
