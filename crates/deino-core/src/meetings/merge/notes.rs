//! The merged meeting's notes: each part's notes under a heading with its
//! title, in time order. Pure; the merge writes the result in its
//! transaction.

use chrono::{DateTime, Utc};
use deino_doc::{Block, Doc, Flavor, HeadingAttrs, Inline, to_markdown};
use deino_domain::MeetingId;

use crate::notes::StoredNotes;

/// The heading level each part's notes go under.
const PART_HEADING_LEVEL: u8 = 2;

/// One part's notes, as the merge reads them.
#[derive(Clone, Debug, PartialEq)]
pub struct PartNotes {
    /// The part's title, for its heading.
    pub title: String,
    /// Its notes, if any were saved.
    pub notes: Option<StoredNotes>,
}

/// The target's combined notes, or `None` when they need no change: no
/// other part has notes worth keeping.
///
/// With notes in two or more parts, each goes under a heading with its
/// title, so nothing reads as if written in another meeting. With notes in
/// a single absorbed part, they keep that heading too.
pub fn combine(target: &MeetingId, parts: &[PartNotes], at: DateTime<Utc>) -> Option<StoredNotes> {
    let written: Vec<(&str, &StoredNotes)> = parts
        .iter()
        .filter_map(|part| {
            part.notes
                .as_ref()
                .map(|notes| (part.title.as_str(), notes))
        })
        .filter(|(_, notes)| !notes.doc.is_blank())
        .collect();
    let target_only = written.iter().all(|(_, notes)| notes.meeting_id == *target);
    if target_only {
        return None;
    }
    let mut content = Vec::new();
    for (title, notes) in &written {
        content.push(heading(title));
        content.extend(notes.doc.content.iter().cloned());
    }
    let doc = Doc { content };
    let created_at = parts
        .iter()
        .filter_map(|part| part.notes.as_ref())
        .find(|notes| notes.meeting_id == *target)
        .map_or(at, |notes| notes.created_at);
    Some(StoredNotes {
        meeting_id: target.clone(),
        markdown: to_markdown(&doc, Flavor::Readable),
        doc,
        created_at,
        updated_at: at,
    })
}

/// A heading reading `title`.
fn heading(title: &str) -> Block {
    Block::Heading {
        attrs: HeadingAttrs {
            level: PART_HEADING_LEVEL,
        },
        content: vec![Inline::text(title, Vec::new())],
    }
}

#[cfg(test)]
mod tests {
    use deino_testkit::at;

    use super::*;

    fn notes(meeting: &str, text: &str) -> StoredNotes {
        let doc = Doc {
            content: vec![Block::paragraph(vec![Inline::text(text, Vec::new())])],
        };
        StoredNotes {
            meeting_id: MeetingId::new(meeting),
            markdown: to_markdown(&doc, Flavor::Readable),
            doc,
            created_at: at("2026-10-01T09:00:00Z"),
            updated_at: at("2026-10-01T10:00:00Z"),
        }
    }

    fn part(title: &str, saved: Option<StoredNotes>) -> PartNotes {
        PartNotes {
            title: title.into(),
            notes: saved,
        }
    }

    #[test]
    fn notes_of_several_parts_go_under_their_titles_in_order() {
        let target = MeetingId::new("a");
        let parts = [
            part("Kickoff", Some(notes("a", "Scope agreed"))),
            part("Follow-up", None),
            part("Wrap-up", Some(notes("c", "Ship Friday"))),
        ];

        let merged = combine(&target, &parts, at("2026-10-12T09:00:00Z")).unwrap();

        assert_eq!(
            merged.markdown,
            "## Kickoff\n\nScope agreed\n\n## Wrap-up\n\nShip Friday\n"
        );
        assert_eq!(merged.meeting_id, target);
        assert_eq!(
            merged.created_at,
            at("2026-10-01T09:00:00Z"),
            "the target's first save is kept"
        );
        assert_eq!(merged.updated_at, at("2026-10-12T09:00:00Z"));
    }

    #[test]
    fn the_targets_notes_alone_stay_as_they_are() {
        let target = MeetingId::new("a");
        let parts = [
            part("Kickoff", Some(notes("a", "Scope"))),
            part("Follow-up", None),
        ];

        assert_eq!(combine(&target, &parts, at("2026-10-12T09:00:00Z")), None);
    }
}
