//! [`ItemQuery`]: an [`ActionItemFilter`] resolved into the list query's
//! parameters. Pure: the clock, the time zone and "Me" come in as values,
//! so the day boundaries ("overdue", "this week") are tested here and the
//! SQL only compares instants.

use chrono::{DateTime, Datelike, Days, NaiveDate, Utc};
use deino_domain::{ActionItemStatus, CompanyId, FolderId, LocalZone, MeetingId, PersonId, TagId};

use super::{ActionItemFilter, AssigneeScope, DateRange, DueFilter, StatusFilter};

/// How the list query reaches its rows: through the index that narrows
/// them most. The other conditions filter what it finds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessPath {
    /// One meeting's items (`idx_action_items_meeting_id`).
    Meeting,
    /// One person's items (`idx_action_items_assignee_status_due_at`).
    Person,
    /// Items by status (`idx_action_items_status_due_at`).
    Status,
}

/// The list query's parameters; `None` and `false` do not filter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemQuery {
    /// The statuses to include (the same one twice for a single status).
    pub statuses: [ActionItemStatus; 2],
    /// Only this person's items.
    pub person: Option<PersonId>,
    /// Only items that have an assignee.
    pub assigned_only: bool,
    /// Not this person's items.
    pub not_person: Option<PersonId>,
    /// Only items without an assignee.
    pub unassigned_only: bool,
    /// Only this meeting's items.
    pub meeting: Option<MeetingId>,
    /// Only items of meetings with this tag.
    pub tag: Option<TagId>,
    /// Only items of meetings with someone from this company, or assigned to
    /// someone there.
    pub company: Option<CompanyId>,
    /// Only items of meetings in this folder or below it.
    pub folder: Option<FolderId>,
    /// Due at or after this instant.
    pub due_from: Option<DateTime<Utc>>,
    /// Due before this instant.
    pub due_before: Option<DateTime<Utc>>,
    /// Only items without a due date.
    pub undated_only: bool,
    /// Held (or created, when standalone) at or after this instant.
    pub held_from: Option<DateTime<Utc>>,
    /// Held before this instant.
    pub held_before: Option<DateTime<Utc>>,
}

/// What a filter is resolved against.
#[derive(Clone, Copy, Debug)]
pub struct QueryContext<'a> {
    /// The user's person, if the profile has created one.
    pub me: Option<PersonId>,
    /// The user's local day now.
    pub today: NaiveDate,
    /// The user's time zone.
    pub zone: &'a dyn LocalZone,
}

impl ItemQuery {
    /// The query for `filter`, or `None` when nothing can match (the user's
    /// own items before a "Me" person exists).
    pub fn resolve(filter: &ActionItemFilter, context: QueryContext<'_>) -> Option<Self> {
        let mut query = Self::with_status(filter.status, filter.due);
        query.scope(filter.assignee, context.me)?;
        query.meeting.clone_from(&filter.meeting_id);
        query.tag = filter.tag_id;
        query.company = filter.company_id;
        query.folder = filter.folder_id;
        if let Some(due) = filter.due {
            query.due_window(due, context);
        }
        if let Some(range) = filter.date_range {
            query.held_window(range, context.zone);
        }
        Some(query)
    }

    /// The index the query should use.
    pub fn access_path(&self) -> AccessPath {
        if self.meeting.is_some() {
            AccessPath::Meeting
        } else if self.person.is_some() {
            AccessPath::Person
        } else {
            AccessPath::Status
        }
    }

    /// No condition but the statuses; overdue items are always open ones.
    fn with_status(status: StatusFilter, due: Option<DueFilter>) -> Self {
        let statuses = match (status, due) {
            (_, Some(DueFilter::Overdue)) | (StatusFilter::Open, _) => [ActionItemStatus::Open; 2],
            (StatusFilter::Done, _) => [ActionItemStatus::Done; 2],
            (StatusFilter::Any, _) => [ActionItemStatus::Open, ActionItemStatus::Done],
        };
        Self {
            statuses,
            person: None,
            assigned_only: false,
            not_person: None,
            unassigned_only: false,
            meeting: None,
            tag: None,
            company: None,
            folder: None,
            due_from: None,
            due_before: None,
            undated_only: false,
            held_from: None,
            held_before: None,
        }
    }

    /// Applies the assignee scope; `None` when it needs a "Me" that is missing.
    fn scope(&mut self, scope: AssigneeScope, me: Option<PersonId>) -> Option<()> {
        match scope {
            AssigneeScope::Me => self.person = Some(me?),
            AssigneeScope::Person { person_id } => self.person = Some(person_id),
            AssigneeScope::Others => {
                self.assigned_only = true;
                self.not_person = me;
            }
            AssigneeScope::Unassigned => self.unassigned_only = true,
            AssigneeScope::Anyone => {}
        }
        Some(())
    }

    fn due_window(&mut self, due: DueFilter, context: QueryContext<'_>) {
        let today = context.today;
        let start = |day: NaiveDate| Some(context.zone.start_of(day));
        match due {
            DueFilter::Overdue => self.due_before = start(today),
            DueFilter::Today => {
                self.due_from = start(today);
                self.due_before = start(today + Days::new(1));
            }
            DueFilter::ThisWeek => {
                self.due_from = start(today);
                self.due_before = start(next_monday(today));
            }
            DueFilter::NoDate => self.undated_only = true,
        }
    }

    fn held_window(&mut self, range: DateRange, zone: &dyn LocalZone) {
        self.held_from = range.from.map(|day| zone.start_of(day));
        self.held_before = range.to.map(|day| zone.start_of(day + Days::new(1)));
    }
}

/// The Monday after `day`'s week: weeks run Monday to Sunday (ISO 8601).
pub fn next_monday(day: NaiveDate) -> NaiveDate {
    let into_week = u64::from(day.weekday().num_days_from_monday());
    day + Days::new(7 - into_week)
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
