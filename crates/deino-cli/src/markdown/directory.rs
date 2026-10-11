//! People, companies, tags, folders and calendar events as Markdown lists.

use deino_core::query::{EventView, PersonView};

use super::{counted, line, when};
use crate::request::{CompanyList, EventList, FolderList, Markdown, PeopleList, TagList};

/// A titled bullet list, or `empty` when there are no rows.
fn list(title: &str, rows: Vec<String>, empty: &str) -> String {
    let mut out = String::new();
    if rows.is_empty() {
        line(&mut out, empty);
        return out;
    }
    line(&mut out, format!("# {title} ({})\n", rows.len()));
    for row in rows {
        line(&mut out, format!("- {row}"));
    }
    out
}

impl Markdown for PeopleList {
    fn markdown(&self) -> String {
        let rows = self.people.iter().map(person).collect();
        list("People", rows, "No people match.")
    }
}

fn person(person: &PersonView) -> String {
    let mut parts = vec![format!("**{}**", person.name)];
    if person.is_me {
        parts.push("me".to_owned());
    }
    parts.extend(person.email.clone());
    let role = [person.title.clone(), person.company.clone()];
    let role: Vec<String> = role.into_iter().flatten().collect();
    if !role.is_empty() {
        parts.push(role.join(", "));
    }
    parts.push(format!("`{}`", person.id));
    parts.join(" · ")
}

impl Markdown for CompanyList {
    fn markdown(&self) -> String {
        let rows = self
            .companies
            .iter()
            .map(|company| {
                let domain = company.domain.as_deref().unwrap_or("no domain");
                format!(
                    "**{}** · {domain} · {}",
                    company.name,
                    counted(company.people, "person", "people")
                )
            })
            .collect();
        list("Companies", rows, "No companies yet.")
    }
}

impl Markdown for TagList {
    fn markdown(&self) -> String {
        let rows = self
            .tags
            .iter()
            .map(|tag| {
                let about = if tag.description.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", tag.description)
                };
                format!(
                    "**{}** · {}{about}",
                    tag.name,
                    counted(tag.meetings, "meeting", "meetings")
                )
            })
            .collect();
        list("Tags", rows, "No tags yet.")
    }
}

impl Markdown for FolderList {
    fn markdown(&self) -> String {
        let rows = self
            .folders
            .iter()
            .map(|folder| {
                format!(
                    "**{}** · {}",
                    folder.path,
                    counted(folder.meetings, "meeting", "meetings")
                )
            })
            .collect();
        list("Folders", rows, "No folders yet.")
    }
}

impl Markdown for EventList {
    fn markdown(&self) -> String {
        let rows = self.events.iter().map(event).collect();
        list(
            "Calendar events",
            rows,
            "No calendar events in this window.",
        )
    }
}

fn event(event: &EventView) -> String {
    let mut parts = vec![
        format!("{}–{}", when(&event.start), event.end.format("%H:%M")),
        format!("**{}**", event.title),
    ];
    parts.extend(event.platform.clone());
    if let Some(rsvp) = &event.my_rsvp {
        parts.push(format!("RSVP {rsvp}"));
    }
    if !event.attendees.is_empty() {
        parts.push(format!("with {}", event.attendees.join(", ")));
    }
    if event.external {
        parts.push("external".to_owned());
    }
    parts.join(" · ")
}
