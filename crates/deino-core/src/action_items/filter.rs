//! [`ActionItemFilter`]: what the Action Items page asks for, as the user
//! chose it (Circleback §8 "Filters"). Saved views store one; the list
//! query resolves it against the clock, the time zone and "Me"
//! ([`ItemQuery::resolve`](super::ItemQuery::resolve)).

use chrono::NaiveDate;
use deino_domain::{CompanyId, FolderId, MeetingId, PersonId, TagId};
use serde::{Deserialize, Serialize};

/// Whose items to show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssigneeScope {
    /// The user's own items (the "Me" person's).
    #[default]
    Me,
    /// Items assigned to someone other than the user.
    Others,
    /// One person's items.
    Person {
        /// The person.
        #[serde(rename = "personId")]
        person_id: PersonId,
    },
    /// Items nobody owns.
    Unassigned,
    /// Every item.
    Anyone,
}

/// Which statuses to show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatusFilter {
    /// Still to do.
    #[default]
    Open,
    /// Done.
    Done,
    /// Both.
    Any,
}

/// Which due dates to show, in the user's local days.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DueFilter {
    /// Open items due before today.
    Overdue,
    /// Due today.
    Today,
    /// Due from today to the end of the week (Sunday).
    ThisWeek,
    /// Without a due date.
    NoDate,
}

/// Inclusive local days the item's meeting took place in (its creation day
/// for a standalone item). Either end may be open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DateRange {
    /// The first day, if bounded.
    pub from: Option<NaiveDate>,
    /// The last day, if bounded.
    pub to: Option<NaiveDate>,
}

/// One Action Items filter. Every condition narrows the list; an absent one
/// does not filter. The default is the page's default view: the user's own
/// open items.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ActionItemFilter {
    /// Whose items.
    pub assignee: AssigneeScope,
    /// Which statuses.
    pub status: StatusFilter,
    /// Only this meeting's items.
    pub meeting_id: Option<MeetingId>,
    /// Only items of meetings with this tag.
    pub tag_id: Option<TagId>,
    /// Only items of meetings with someone from this company, or assigned
    /// to someone there.
    pub company_id: Option<CompanyId>,
    /// Only items of meetings in this folder or any folder inside it.
    pub folder_id: Option<FolderId>,
    /// Only items due in this window.
    pub due: Option<DueFilter>,
    /// Only items of meetings held in this range.
    pub date_range: Option<DateRange>,
}

impl ActionItemFilter {
    /// Every item of one meeting, whoever owns it and whatever its status:
    /// what the meeting's Notes tab lists.
    pub fn meeting(meeting: MeetingId) -> Self {
        Self {
            assignee: AssigneeScope::Anyone,
            status: StatusFilter::Any,
            meeting_id: Some(meeting),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_default_filter_is_my_open_items() {
        let filter = ActionItemFilter::default();

        assert_eq!(filter.assignee, AssigneeScope::Me);
        assert_eq!(filter.status, StatusFilter::Open);
    }

    #[test]
    fn a_saved_filter_missing_fields_reads_with_their_defaults() {
        let filter: ActionItemFilter =
            serde_json::from_value(json!({ "due": "this_week" })).unwrap();

        assert_eq!(filter.due, Some(DueFilter::ThisWeek));
        assert_eq!(filter.assignee, AssigneeScope::Me);
    }

    #[test]
    fn scopes_are_tagged_by_kind_with_camel_case_fields() {
        let person: PersonId = "0199c6a0-0000-7000-8000-000000000001".parse().unwrap();

        let json = serde_json::to_value(AssigneeScope::Person { person_id: person }).unwrap();

        assert_eq!(
            json,
            json!({ "kind": "person", "personId": person.to_string() })
        );
    }
}
