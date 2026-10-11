//! The full account export's tables (EXP-02): meetings, action items and
//! people, and one insight's answers across meetings (AI-06), as RFC 4180
//! CSV that Numbers, Excel and Sheets open as is.
//!
//! Cells that a spreadsheet would run as a formula (`=`, `+`, `-`, `@`) are
//! prefixed with `'`, so a meeting title can never become one.

use std::collections::BTreeMap;

use crate::{InsightTableExport, MeetingExport, labels};

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
    table_of(&header, rows)
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
    table_of(&header, rows)
}

/// One row per insight answer of every meeting (the account export's
/// `insights.csv`): the meeting, the insight, the answer on one line, who
/// said it and when.
pub fn insights(exports: &[MeetingExport]) -> String {
    let header = [
        "meeting_id",
        "meeting_title",
        "insight",
        "answer",
        "speaker",
        "time",
    ];
    let rows = exports.iter().flat_map(|export| {
        export.insights.iter().flat_map(move |insight| {
            insight.answers.iter().map(move |answer| {
                vec![
                    export.id.to_string(),
                    export.title.clone(),
                    insight.name.clone(),
                    answer.line(),
                    answer.speaker.clone().unwrap_or_default(),
                    answer.at.map(labels::moment).unwrap_or_default(),
                ]
            })
        })
    });
    table_of(&header, rows)
}

/// One row per answer of one insight, newest meeting first: the meeting,
/// its date, a column per field (or one "Answer"), who said it, when, and a
/// link that opens the meeting. Headers are the user's own field names.
pub fn insight_table(table: &InsightTableExport) -> String {
    let mut header: Vec<&str> = vec!["Meeting", "Date"];
    header.extend(table.columns.iter().map(String::as_str));
    header.extend(["Speaker", "Time", "Link"]);
    let rows = table.rows.iter().map(|row| {
        let mut cells = vec![
            row.meeting_title.clone(),
            labels::iso_local(&row.started_at),
        ];
        cells.extend(row.cells.iter().cloned());
        cells.push(row.speaker.clone().unwrap_or_default());
        cells.push(row.at.map(labels::moment).unwrap_or_default());
        cells.push(format!("{}{}", crate::model::LINK_PREFIX, row.meeting_id));
        cells
    });
    table_of(&header, rows)
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
    table_of(&["name", "email", "meetings", "last_met"], rows)
}

/// A person's attendance across the exported meetings.
struct Tally {
    name: String,
    email: String,
    meetings: u32,
    last_met: chrono::NaiveDate,
}

/// The header and rows as CSV; header cells are quoted like any other, since
/// an insight's are the user's own field names.
fn table_of(header: &[&str], rows: impl Iterator<Item = Vec<String>>) -> String {
    let mut out = header
        .iter()
        .map(|cell| cell_of(cell))
        .collect::<Vec<_>>()
        .join(",");
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
    fn the_fixture_account_exports_four_tables() {
        let exports = fixture_exports();
        insta::assert_snapshot!("csv_meetings", meetings(&exports));
        insta::assert_snapshot!("csv_action_items", action_items(&exports));
        insta::assert_snapshot!("csv_people", people(&exports));
        insta::assert_snapshot!("csv_insights", insights(&exports));
    }

    #[test]
    fn cells_are_quoted_and_never_formulas() {
        assert_eq!(cell_of("plain"), "plain");
        assert_eq!(cell_of("a, \"b\"\nc"), "\"a, \"\"b\"\"\nc\"");
        assert_eq!(cell_of("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(cell_of("-1 day"), "'-1 day");
    }

    #[test]
    fn an_insight_table_has_a_column_per_field_and_attributes_each_answer() {
        use chrono::TimeZone;
        use deino_domain::{MeetingId, Millis};

        use crate::InsightRowExport;

        let offset = chrono::FixedOffset::west_opt(7 * 3_600).unwrap();
        let row = |cells: [&str; 2], speaker: Option<&str>, at: Option<u32>| InsightRowExport {
            meeting_id: MeetingId::new("meeting-a"),
            meeting_title: "Brightkite discovery, call 2".into(),
            started_at: offset.with_ymd_and_hms(2026, 9, 30, 10, 0, 0).unwrap(),
            cells: cells.map(str::to_owned).to_vec(),
            speaker: speaker.map(str::to_owned),
            at: at.map(Millis),
        };
        let table = InsightTableExport {
            columns: vec!["Budget, USD".into(), "=Score".into()],
            rows: vec![
                row(["40000", "4"], Some("Marcus Webb"), Some(271_000)),
                row(["", "2"], None, None),
            ],
        };

        insta::assert_snapshot!("csv_insight_table", insight_table(&table));
    }
}
