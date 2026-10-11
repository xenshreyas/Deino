//! [`ActionItemService`]: the rules behind the Action Items page, the
//! Notes tab's list and the "Today" digest. No SQL and no Tauri: ports in,
//! time from the [`Clock`], days from the [`LocalZone`].

use std::sync::Arc;

use deino_domain::{ActionItemId, ActionItemOrigin, ActionItemStatus, Clock, LocalZone, PersonId};
use serde::Serialize;

use super::{
    ActionItem, ActionItemBatch, ActionItemChange, ActionItemChangeKind, ActionItemEdit,
    ActionItemError, ActionItemEvents, ActionItemFilter, ActionItemReader, ActionItemRepo,
    ActionItemView, DueFilter, FilterOptions, ItemQuery, NewActionItem, OptionSources,
    QueryContext, valid_description, valid_title,
};
use crate::people::MeRepo;

/// The ports the service works through.
#[derive(Clone)]
pub struct ActionItemDeps {
    /// Single-item writes.
    pub items: Arc<dyn ActionItemRepo>,
    /// Many-item writes.
    pub batch: Arc<dyn ActionItemBatch>,
    /// The list and single views.
    pub reader: Arc<dyn ActionItemReader>,
    /// Who "Me" is.
    pub me: Arc<dyn MeRepo>,
    /// People, companies, tags and folders for the filter bar.
    pub options: OptionSources,
    /// Where changes are announced.
    pub events: Arc<dyn ActionItemEvents>,
    /// The time.
    pub clock: Arc<dyn Clock>,
    /// The user's time zone.
    pub zone: Arc<dyn LocalZone>,
}

/// The user's open items that need attention today: the "Today" digest
/// (after `LokalBot`'s, community.md §5) and the 09:00 reminder.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TodayDigest {
    /// Open items due before today, oldest first.
    pub overdue: Vec<ActionItemView>,
    /// Open items due today.
    pub due_today: Vec<ActionItemView>,
}

impl TodayDigest {
    /// True when nothing needs attention.
    pub fn is_empty(&self) -> bool {
        self.overdue.is_empty() && self.due_today.is_empty()
    }
}

/// Lists, creates and changes action items.
pub struct ActionItemService {
    deps: ActionItemDeps,
}

impl ActionItemService {
    /// The service over its ports.
    pub fn new(deps: ActionItemDeps) -> Self {
        Self { deps }
    }

    /// The items `filter` matches, dated ones soonest first.
    pub async fn list(
        &self,
        filter: &ActionItemFilter,
    ) -> Result<Vec<ActionItemView>, ActionItemError> {
        let me = self.me().await?;
        let context = QueryContext {
            me,
            today: self.deps.zone.day_of(self.deps.clock.now()),
            zone: self.deps.zone.as_ref(),
        };
        match ItemQuery::resolve(filter, context) {
            Some(query) => Ok(self.deps.reader.list(&query).await?),
            None => Ok(Vec::new()),
        }
    }

    /// The user's overdue and due-today open items.
    pub async fn today(&self) -> Result<TodayDigest, ActionItemError> {
        let mine = |due| ActionItemFilter {
            due: Some(due),
            ..ActionItemFilter::default()
        };
        Ok(TodayDigest {
            overdue: self.list(&mine(DueFilter::Overdue)).await?,
            due_today: self.list(&mine(DueFilter::Today)).await?,
        })
    }

    /// Adds an item the user wrote.
    pub async fn create(&self, new: NewActionItem) -> Result<ActionItemView, ActionItemError> {
        if let Some(person) = new.assignee_person_id {
            self.require_person(person).await?;
        }
        let now = self.deps.clock.now();
        let mut item = ActionItem {
            id: ActionItemId::generate(self.deps.clock.as_ref()),
            meeting_id: new.meeting_id,
            title: valid_title(&new.title)?,
            description: valid_description(&new.description)?,
            assignee_person_id: new.assignee_person_id,
            due_at: None,
            status: ActionItemStatus::Open,
            priority: new.priority,
            source_segment_id: None,
            completed_at: None,
            created_by: ActionItemOrigin::User,
            created_at: now,
            updated_at: now,
        };
        ActionItemEdit::Due { date: new.due }.apply(&mut item, self.deps.zone.as_ref())?;
        self.deps.items.create(&item).await?;
        self.announce(ActionItemChangeKind::Created, vec![item.id]);
        self.view(item.id).await
    }

    /// Applies one edit to one item.
    pub async fn update(
        &self,
        id: ActionItemId,
        edit: &ActionItemEdit,
    ) -> Result<ActionItemView, ActionItemError> {
        let mut item = self.find(id).await?;
        let now = self.deps.clock.now();
        if let ActionItemEdit::Status { status } = edit {
            self.deps.items.set_status(&id, *status, now).await?;
        } else {
            self.check_people(edit).await?;
            edit.apply(&mut item, self.deps.zone.as_ref())?;
            item.updated_at = now;
            self.deps.items.update(&item).await?;
        }
        self.announce(change_kind(edit), vec![id]);
        self.view(id).await
    }

    /// Applies one bulk edit (status, assignee, due day or priority) to
    /// every item in `ids`, in one transaction. Returns how many changed.
    pub async fn bulk_update(
        &self,
        ids: &[ActionItemId],
        edit: &ActionItemEdit,
    ) -> Result<u64, ActionItemError> {
        if !edit.is_bulk() {
            return Err(ActionItemError::invalid(
                "edit",
                "cannot change many items' wording at once",
            ));
        }
        let now = self.deps.clock.now();
        let changed = if let ActionItemEdit::Status { status } = edit {
            self.deps.batch.set_status_many(ids, *status, now).await?
        } else {
            self.check_people(edit).await?;
            let mut items = Vec::with_capacity(ids.len());
            for id in ids {
                let mut item = self.find(*id).await?;
                edit.apply(&mut item, self.deps.zone.as_ref())?;
                item.updated_at = now;
                items.push(item);
            }
            self.deps.batch.update_many(&items).await?
        };
        self.announce(change_kind(edit), ids.to_vec());
        Ok(changed)
    }

    /// Deletes every item in `ids`. Returns how many existed.
    pub async fn delete(&self, ids: &[ActionItemId]) -> Result<u64, ActionItemError> {
        let deleted = self.deps.batch.delete_many(ids).await?;
        self.announce(ActionItemChangeKind::Deleted, ids.to_vec());
        Ok(deleted)
    }

    /// Completes every open item of the user's ("Mark all mine done").
    pub async fn mark_all_mine_done(&self) -> Result<u64, ActionItemError> {
        let mine = self.list(&ActionItemFilter::default()).await?;
        let ids: Vec<ActionItemId> = mine.iter().map(|item| item.id).collect();
        self.bulk_update(
            &ids,
            &ActionItemEdit::Status {
                status: ActionItemStatus::Done,
            },
        )
        .await
    }

    /// The people, companies, tags and folders to filter by.
    pub async fn filter_options(&self) -> Result<FilterOptions, ActionItemError> {
        Ok(self.deps.options.load().await?)
    }

    async fn me(&self) -> Result<Option<PersonId>, ActionItemError> {
        Ok(self.deps.me.me().await?.map(|person| person.id))
    }

    async fn find(&self, id: ActionItemId) -> Result<ActionItem, ActionItemError> {
        self.deps
            .items
            .find(&id)
            .await?
            .ok_or(ActionItemError::NotFound(id))
    }

    async fn view(&self, id: ActionItemId) -> Result<ActionItemView, ActionItemError> {
        self.deps
            .reader
            .view(&id)
            .await?
            .ok_or(ActionItemError::NotFound(id))
    }

    /// Fails unless an assignee the edit names exists.
    async fn check_people(&self, edit: &ActionItemEdit) -> Result<(), ActionItemError> {
        match edit {
            ActionItemEdit::Assignee {
                person_id: Some(person),
            } => self.require_person(*person).await,
            _ => Ok(()),
        }
    }

    async fn require_person(&self, person: PersonId) -> Result<(), ActionItemError> {
        match self.deps.options.people.find(&person).await? {
            Some(_) => Ok(()),
            None => Err(ActionItemError::PersonNotFound(person)),
        }
    }

    fn announce(&self, kind: ActionItemChangeKind, ids: Vec<ActionItemId>) {
        self.deps.events.changed(&ActionItemChange { kind, ids });
    }
}

/// How an edit is announced.
fn change_kind(edit: &ActionItemEdit) -> ActionItemChangeKind {
    match edit {
        ActionItemEdit::Status {
            status: ActionItemStatus::Done,
        } => ActionItemChangeKind::Completed,
        ActionItemEdit::Status {
            status: ActionItemStatus::Open,
        } => ActionItemChangeKind::Reopened,
        _ => ActionItemChangeKind::Updated,
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
