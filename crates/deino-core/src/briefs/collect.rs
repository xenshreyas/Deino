//! Turning what [`BriefSources`](super::BriefSources) read into prompt
//! [`Source`]s and the event header: pure, so the prompt's wording is tested
//! without a database. Dates are absolute (never "3 days ago"), so the
//! inputs' fingerprint only changes when the data does.

use std::collections::BTreeMap;

use deino_domain::{LocalZone, PersonId};

use super::{BriefCitation, Source};
use crate::action_items::ActionItem;
use crate::calendar::CalendarEvent;
use crate::companies::Company;
use crate::entities::EntityDigest;
use crate::people::Person;

/// The most characters of one meeting's summary or notes in the prompt.
const MEETING_CHARS: usize = 1_200;
/// The most characters of a person's or company's summary.
const SUMMARY_CHARS: usize = 600;
/// The most characters of the invitation's description.
const DESCRIPTION_CHARS: usize = 800;
/// How a date reads in the prompt: "Wednesday 30 September 2026".
const DATE_FORMAT: &str = "%A %-d %B %Y";

/// The event block: title, local start time and the attendees, each with
/// what Deino knows (title, company).
pub fn event_header(
    event: &CalendarEvent,
    people: &[Person],
    companies: &[Company],
    zone: &dyn LocalZone,
) -> String {
    let start = zone.local(event.start_at);
    let mut lines = vec![
        format!("Title: {}", event.title.trim()),
        format!("When: {}", start.format("%A %-d %B %Y, %H:%M")),
        "Attendees:".to_owned(),
    ];
    let known: BTreeMap<&str, &Person> = people
        .iter()
        .filter_map(|person| Some((person.email.as_ref()?.as_str(), person)))
        .collect();
    for attendee in &event.attendees {
        let email = attendee.email.as_deref().unwrap_or_default();
        let line = match known.get(email.to_ascii_lowercase().as_str()) {
            Some(person) => person_line(person, companies),
            None => attendee.name.clone().unwrap_or_else(|| email.to_owned()),
        };
        lines.push(format!("- {line}"));
    }
    lines.join("\n")
}

/// "Maya Chen, Staff Engineer, Acme".
fn person_line(person: &Person, companies: &[Company]) -> String {
    let company = person
        .company_id
        .and_then(|id| companies.iter().find(|company| company.id == id));
    [
        Some(person.name.as_str()),
        person.title.as_deref(),
        company.map(|c| c.name.as_str()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ")
}

/// An earlier meeting: its title and date, then its summary, else the
/// user's notes.
pub fn meeting_source(digest: &EntityDigest, last_in_series: bool, zone: &dyn LocalZone) -> Source {
    let date = zone.local(digest.at).format(DATE_FORMAT);
    let series = if last_in_series {
        ", the last meeting of this series"
    } else {
        ""
    };
    let body = digest
        .summary
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .or(digest.notes.as_deref())
        .unwrap_or_default();
    Source {
        citation: BriefCitation::Meeting {
            meeting_id: digest.meeting_id.clone(),
            title: digest.title.clone(),
            at: digest.at,
        },
        heading: format!("Meeting: {} ({date}{series})", digest.title.trim()),
        body: clip(body, MEETING_CHARS),
    }
}

/// An open action item, with its owner and due date; it cites the moment it
/// was agreed when it has one.
pub fn item_source(item: &ActionItem, names: &BTreeMap<PersonId, &str>) -> Source {
    let owner = item
        .assignee_person_id
        .and_then(|id| names.get(&id).copied())
        .unwrap_or("someone");
    let due = item
        .due_at
        .map(|due| format!(", due {}", due.format("%-d %B %Y")))
        .unwrap_or_default();
    let label = item.title.trim().to_owned();
    let citation = match (&item.meeting_id, item.source_segment_id) {
        (Some(meeting_id), Some(segment_id)) => BriefCitation::Moment {
            meeting_id: meeting_id.clone(),
            segment_id,
            label: label.clone(),
        },
        (Some(meeting_id), None) => BriefCitation::Meeting {
            meeting_id: meeting_id.clone(),
            title: label.clone(),
            at: item.created_at,
        },
        (None, _) => BriefCitation::ActionItem {
            action_item_id: item.id,
            label: label.clone(),
        },
    };
    Source {
        citation,
        heading: format!("Open action item for {owner}{due}: {label}"),
        body: clip(&item.description, SUMMARY_CHARS),
    }
}

/// The people's and companies' AI summaries, for those that have one.
pub fn summary_sources(people: &[Person], companies: &[Company]) -> Vec<Source> {
    let people = people.iter().filter_map(|person| {
        let summary = person
            .summary_md
            .as_deref()
            .filter(|s| !s.trim().is_empty())?;
        Some(Source {
            citation: BriefCitation::Person {
                person_id: person.id,
                name: person.name.clone(),
            },
            heading: format!("About {}", person.name),
            body: clip(summary, SUMMARY_CHARS),
        })
    });
    let companies = companies.iter().filter_map(|company| {
        let summary = company
            .summary_md
            .as_deref()
            .filter(|s| !s.trim().is_empty())?;
        Some(Source {
            citation: BriefCitation::Company {
                company_id: company.id,
                name: company.name.clone(),
            },
            heading: format!("About {}", company.name),
            body: clip(summary, SUMMARY_CHARS),
        })
    });
    people.chain(companies).collect()
}

/// The invitation's description, when it has one.
pub fn event_source(event: &CalendarEvent) -> Option<Source> {
    let description = event.description.trim();
    (!description.is_empty()).then(|| Source {
        citation: BriefCitation::Event,
        heading: "Invitation".to_owned(),
        body: clip(description, DESCRIPTION_CHARS),
    })
}

/// At most `limit` characters of `text`, trimmed, with "…" when cut.
fn clip(text: &str, limit: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let cut: String = text.chars().take(limit).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
#[path = "collect_tests.rs"]
mod tests;
