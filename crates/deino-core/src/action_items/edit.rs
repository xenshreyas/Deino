//! Changes the user makes: [`NewActionItem`] (quick add, ⌘K) and
//! [`ActionItemEdit`] (inline edits and bulk actions), with the field rules
//! both share.

use chrono::NaiveDate;
use deino_domain::{ActionItemPriority, ActionItemStatus, LocalZone, MeetingId, NOON, PersonId};
use serde::{Deserialize, Serialize};

use super::{ActionItem, ActionItemError};

/// The longest title, in characters. Extracted titles stay under 80; the
/// user may write more, but a title is still one line.
pub const MAX_TITLE_CHARS: usize = 200;
/// The longest description, in characters.
pub const MAX_DESCRIPTION_CHARS: usize = 2_000;

/// An item the user adds by hand, on a meeting or standalone.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct NewActionItem {
    /// What to do.
    pub title: String,
    /// Context; may be empty.
    pub description: String,
    /// The meeting it belongs to; `None` for a standalone item.
    pub meeting_id: Option<MeetingId>,
    /// Who should do it.
    pub assignee_person_id: Option<PersonId>,
    /// The local day it is due.
    pub due: Option<NaiveDate>,
    /// How urgent it is.
    pub priority: Option<ActionItemPriority>,
}

/// One change to an item, from an inline edit or a bulk action.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, specta::Type)]
#[serde(tag = "field", rename_all = "snake_case")]
pub enum ActionItemEdit {
    /// A new title.
    Title {
        /// What to do.
        title: String,
    },
    /// New context.
    Description {
        /// One or two sentences; may be empty.
        description: String,
    },
    /// A new assignee, or none.
    Assignee {
        /// Who should do it.
        #[serde(rename = "personId")]
        person_id: Option<PersonId>,
    },
    /// A new due day, or none.
    Due {
        /// The local day it is due.
        date: Option<NaiveDate>,
    },
    /// A new priority, or none.
    Priority {
        /// How urgent it is.
        priority: Option<ActionItemPriority>,
    },
    /// Done or open again.
    Status {
        /// The new status.
        status: ActionItemStatus,
    },
}

impl ActionItemEdit {
    /// True for the edits a bulk action may apply to many items at once:
    /// status, assignee, due day and priority (never the wording).
    pub fn is_bulk(&self) -> bool {
        !matches!(self, Self::Title { .. } | Self::Description { .. })
    }

    /// Applies every edit but [`ActionItemEdit::Status`] to `item`'s fields
    /// (status changes keep `completed_at` in step in the repository).
    pub fn apply(
        &self,
        item: &mut ActionItem,
        zone: &dyn LocalZone,
    ) -> Result<(), ActionItemError> {
        match self {
            Self::Title { title } => item.title = valid_title(title)?,
            Self::Description { description } => item.description = valid_description(description)?,
            Self::Assignee { person_id } => item.assignee_person_id = *person_id,
            Self::Due { date } => item.due_at = date.map(|day| zone.at(day, NOON)),
            Self::Priority { priority } => item.priority = *priority,
            Self::Status { .. } => {}
        }
        Ok(())
    }
}

/// A trimmed, non-blank title of at most [`MAX_TITLE_CHARS`].
pub fn valid_title(title: &str) -> Result<String, ActionItemError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(ActionItemError::invalid("title", "cannot be empty"));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(ActionItemError::invalid("title", "is too long"));
    }
    Ok(title.to_owned())
}

/// A trimmed description of at most [`MAX_DESCRIPTION_CHARS`].
pub fn valid_description(description: &str) -> Result<String, ActionItemError> {
    let description = description.trim();
    if description.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(ActionItemError::invalid("description", "is too long"));
    }
    Ok(description.to_owned())
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use deino_domain::{ActionItemId, ActionItemOrigin, FakeClock, FixedZone};
    use serde_json::json;

    use super::*;

    fn item() -> ActionItem {
        let at = DateTime::UNIX_EPOCH;
        ActionItem {
            id: ActionItemId::generate(&FakeClock::new(at)),
            meeting_id: None,
            title: "Draft".into(),
            description: String::new(),
            assignee_person_id: None,
            due_at: None,
            status: ActionItemStatus::Open,
            priority: None,
            source_segment_id: None,
            completed_at: None,
            created_by: ActionItemOrigin::User,
            created_at: at,
            updated_at: at,
        }
    }

    #[test]
    fn a_due_day_is_stored_as_local_noon() {
        let mut task = item();
        let pacific = FixedZone::hours(-7).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 10, 9);

        ActionItemEdit::Due { date }
            .apply(&mut task, &pacific)
            .unwrap();

        assert_eq!(
            task.due_at,
            Some(Utc.with_ymd_and_hms(2026, 10, 9, 19, 0, 0).unwrap())
        );
    }

    #[test]
    fn titles_are_trimmed_and_must_say_something() {
        let mut task = item();
        let zone = FixedZone::utc();

        ActionItemEdit::Title {
            title: "  Send the deck ".into(),
        }
        .apply(&mut task, &zone)
        .unwrap();
        let blank = ActionItemEdit::Title {
            title: "   ".into(),
        }
        .apply(&mut task, &zone);
        let long = ActionItemEdit::Title {
            title: "x".repeat(MAX_TITLE_CHARS + 1),
        }
        .apply(&mut task, &zone);

        assert_eq!(task.title, "Send the deck");
        assert!(matches!(
            blank,
            Err(ActionItemError::Invalid { field: "title", .. })
        ));
        assert!(long.is_err());
    }

    #[test]
    fn bulk_actions_never_rewrite_wording() {
        assert!(
            ActionItemEdit::Status {
                status: ActionItemStatus::Done
            }
            .is_bulk()
        );
        assert!(ActionItemEdit::Assignee { person_id: None }.is_bulk());
        assert!(!ActionItemEdit::Title { title: "x".into() }.is_bulk());
    }

    #[test]
    fn edits_are_tagged_by_field_in_json() {
        let edit: ActionItemEdit =
            serde_json::from_value(json!({ "field": "due", "date": "2026-10-16" })).unwrap();
        let clear: ActionItemEdit =
            serde_json::from_value(json!({ "field": "assignee", "personId": null })).unwrap();

        assert_eq!(
            edit,
            ActionItemEdit::Due {
                date: NaiveDate::from_ymd_opt(2026, 10, 16)
            }
        );
        assert_eq!(clear, ActionItemEdit::Assignee { person_id: None });
    }
}
