//! [`ActionItemQueries`]: action items by status, assignee, due date and
//! meeting, through the same [`ActionItemService`](crate::action_items::ActionItemService)
//! filters as the Action Items page.

use deino_domain::{LocalZone, MeetingId};

use super::link::{meeting_link, moment_link};
use super::{ActionItemSummary, QueryError, ReadPorts, Resolver};
use crate::action_items::{
    ActionItemFilter, ActionItemView, AssigneeScope, DueFilter, StatusFilter,
};

/// How many items a list returns when the caller does not say.
pub const DEFAULT_ITEMS: u32 = 50;
/// The most items one list returns.
pub const MAX_ITEMS: u32 = 500;

/// Whose items to list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Assignee {
    /// Everyone's.
    #[default]
    Anyone,
    /// The user's own.
    Me,
    /// Assigned to someone other than the user.
    Others,
    /// Owned by nobody.
    Unassigned,
    /// One person's: an ID, name or email.
    Person(String),
}

/// Which action items to list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemFilter {
    /// Open, done or both.
    pub status: StatusFilter,
    /// Whose.
    pub assignee: Assignee,
    /// Which due dates.
    pub due: Option<DueFilter>,
    /// Only this meeting's.
    pub meeting: Option<MeetingId>,
    /// The most to return; 0 means [`DEFAULT_ITEMS`].
    pub limit: u32,
}

impl Default for ItemFilter {
    /// Everyone's open items.
    fn default() -> Self {
        Self {
            status: StatusFilter::Open,
            assignee: Assignee::Anyone,
            due: None,
            meeting: None,
            limit: 0,
        }
    }
}

impl ItemFilter {
    fn limit(&self) -> usize {
        let limit = match self.limit {
            0 => DEFAULT_ITEMS,
            limit => limit.min(MAX_ITEMS),
        };
        usize::try_from(limit).unwrap_or(usize::MAX)
    }
}

/// Reads action items.
pub struct ActionItemQueries {
    ports: ReadPorts,
}

impl ActionItemQueries {
    /// The queries over `ports`.
    pub fn new(ports: ReadPorts) -> Self {
        Self { ports }
    }

    /// The items `filter` matches, dated ones soonest first.
    pub async fn list(&self, filter: &ItemFilter) -> Result<Vec<ActionItemSummary>, QueryError> {
        let resolved = ActionItemFilter {
            assignee: self.scope(&filter.assignee).await?,
            status: filter.status,
            meeting_id: filter.meeting.clone(),
            due: filter.due,
            ..ActionItemFilter::default()
        };
        let views = self.ports.action_items.list(&resolved).await?;
        let zone = self.ports.meeting.zone.as_ref();
        Ok(views
            .into_iter()
            .take(filter.limit())
            .map(|view| summarize(view, zone))
            .collect())
    }

    async fn scope(&self, assignee: &Assignee) -> Result<AssigneeScope, QueryError> {
        Ok(match assignee {
            Assignee::Anyone => AssigneeScope::Anyone,
            Assignee::Me => AssigneeScope::Me,
            Assignee::Others => AssigneeScope::Others,
            Assignee::Unassigned => AssigneeScope::Unassigned,
            Assignee::Person(reference) => AssigneeScope::Person {
                person_id: Resolver::new(&self.ports).person(reference).await?,
            },
        })
    }
}

/// An item as the library shows it: its due day in the user's zone and a
/// link to the moment it was said.
pub fn summarize(view: ActionItemView, zone: &dyn LocalZone) -> ActionItemSummary {
    let link = view
        .meeting_id
        .as_ref()
        .map(|meeting| match view.source_start {
            Some(at) => moment_link(meeting, at),
            None => meeting_link(meeting),
        });
    ActionItemSummary {
        id: view.id,
        title: view.title,
        description: view.description,
        status: view.status,
        assignee: view.assignee_name,
        assignee_is_me: view.assignee_is_me,
        due: view.due_at.map(|due| zone.day_of(due)),
        meeting_id: view.meeting_id,
        meeting_title: view.meeting_title,
        link,
    }
}
