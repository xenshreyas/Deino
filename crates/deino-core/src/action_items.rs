//! Action items (AI-05): commitments extracted from meetings, and standalone
//! to-dos, with the Action Items page's filters, saved views, bulk edits and
//! the morning reminder.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs`, `view.rs` | [`ActionItem`] (a row) and [`ActionItemView`] (a list row with its meeting and assignee) |
//! | `filter.rs`, `query.rs` | [`ActionItemFilter`] (what the user picked) resolved into an [`ItemQuery`] against the clock, the time zone and "Me" |
//! | `repo.rs`, `batch_repo.rs`, `list_repo.rs` | the ports [`ActionItemRepo`], [`ActionItemBatch`], [`ActionItemReader`] and [`SqliteActionItemRepo`] |
//! | `edit.rs` | [`NewActionItem`], [`ActionItemEdit`] and the field rules |
//! | `service.rs` | [`ActionItemService`]: list, today, create, update, bulk, delete, "mark all mine done" |
//! | `views.rs`, `view_service.rs` | saved views ([`SavedView`], [`SavedViewService`]) |
//! | `options.rs` | [`FilterOptions`]: people, companies, tags and folders to filter by |
//! | `events.rs` | the [`ActionItemEvents`] port (INT-03, AUT-01) |
//! | `extraction.rs`, `extraction/` | the `extract_action_items` stage's rules |
//! | `reminders.rs` | [`DailyReminder`], the morning notification |
//!
//! Due dates are local days, stored as noon local time so they read as the
//! same day in any nearby time zone (ADR 0028). `external_refs_json` is
//! INT-03's sync payload, which this module neither reads nor writes.

#[cfg(test)]
mod action_item_tests;
mod batch_repo;
mod edit;
mod error;
mod events;
pub mod extraction;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod filter;
mod list_repo;
mod model;
mod options;
mod query;
mod reminders;
mod repo;
mod service;
#[cfg(test)]
mod test_world;
mod view;
mod view_service;
mod views;

pub use batch_repo::ActionItemBatch;
pub use edit::{
    ActionItemEdit, MAX_DESCRIPTION_CHARS, MAX_TITLE_CHARS, NewActionItem, valid_description,
    valid_title,
};
pub use error::ActionItemError;
pub use events::{
    ActionItemChange, ActionItemChangeKind, ActionItemEvents, NoActionItemEvents,
    RecordedActionItemEvents,
};
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeActionItemExtractor, FakeActionItemRepo};
pub use filter::{ActionItemFilter, AssigneeScope, DateRange, DueFilter, StatusFilter};
pub use list_repo::ActionItemReader;
pub use model::ActionItem;
pub use options::{
    CompanyOption, FilterOptions, FolderOption, OptionSources, PersonOption, TagOption,
};
pub use query::{AccessPath, ItemQuery, QueryContext, next_monday};
pub use reminders::{
    CATCH_UP_UNTIL, DailyReminder, Notice, Notifier, REMINDER_TIME, ReminderDeps, ReminderSwitch,
    notice,
};
pub use repo::{ActionItemRepo, SqliteActionItemRepo};
pub use service::{ActionItemDeps, ActionItemService, TodayDigest};
pub use view::ActionItemView;
pub use view_service::{MAX_VIEW_NAME_CHARS, SavedViewService};
pub use views::{SavedView, SavedViewInput, SavedViewRepo};
