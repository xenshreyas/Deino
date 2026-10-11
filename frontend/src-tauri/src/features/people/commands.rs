//! Commands of the `people` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::entities::{
    AttendanceService, EntitySummarizer, MeetingPerson, PersonFields, PersonPage, PersonRow,
    PersonService, SummaryOutcome,
};
use deino_core::people::{PeopleService, Person};
use deino_domain::{AppError, MeetingId, PersonId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The People pages' service, as Tauri state.
type People<'a> = State<'a, Arc<PersonService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        search,
        list,
        page,
        create,
        update,
        set_notes,
        merge,
        from_mention,
        refresh_summary,
        attendees,
        add_attendee,
        remove_attendee
    ]
}

/// Up to eight people whose name, a word of it, or email starts with `query`;
/// the first people by name for an empty query.
#[tauri::command]
#[specta::specta]
async fn search(
    people: State<'_, Arc<PeopleService>>,
    query: String,
) -> Result<Vec<Person>, AppError> {
    people.search(&query).await.into_ipc()
}

/// Everyone the user meets, most recently met first.
#[tauri::command]
#[specta::specta]
async fn list(people: People<'_>) -> Result<Vec<PersonRow>, AppError> {
    people.list().await.into_ipc()
}

/// The person page of `id` (or of whom they were merged into).
#[tauri::command]
#[specta::specta]
async fn page(people: People<'_>, id: PersonId) -> Result<PersonPage, AppError> {
    people.page(&id).await.into_ipc()
}

/// Adds a person; `invalid` for a blank name or bad address, `conflict`
/// for an address someone else has.
#[tauri::command]
#[specta::specta]
async fn create(people: People<'_>, fields: PersonFields) -> Result<Person, AppError> {
    people.create(&fields).await.into_ipc()
}

/// Saves the person form.
#[tauri::command]
#[specta::specta]
async fn update(
    people: People<'_>,
    id: PersonId,
    fields: PersonFields,
) -> Result<Person, AppError> {
    people.update(&id, &fields).await.into_ipc()
}

/// Saves the user's notes about a person.
#[tauri::command]
#[specta::specta]
async fn set_notes(people: People<'_>, id: PersonId, notes: String) -> Result<(), AppError> {
    people.set_notes(&id, &notes).await.into_ipc()
}

/// Merges `absorbed` into `survivor`, moving every link; returns the survivor.
#[tauri::command]
#[specta::specta]
async fn merge(
    people: People<'_>,
    survivor: PersonId,
    absorbed: PersonId,
) -> Result<Person, AppError> {
    people.merge(&survivor, &absorbed).await.into_ipc()
}

/// The person called `name`, added if nobody is (an @mention of someone new).
#[tauri::command]
#[specta::specta]
async fn from_mention(people: People<'_>, name: String) -> Result<Person, AppError> {
    people.from_mention(&name).await.into_ipc()
}

/// Rewrites the person's AI summary now.
#[tauri::command]
#[specta::specta]
async fn refresh_summary(
    summaries: State<'_, Arc<EntitySummarizer>>,
    id: PersonId,
) -> Result<SummaryOutcome, AppError> {
    summaries.summarize_person(&id).await.into_ipc()
}

/// The meeting's attendees: the organiser first, the user last.
#[tauri::command]
#[specta::specta]
async fn attendees(
    attendance: State<'_, Arc<AttendanceService>>,
    meeting_id: MeetingId,
) -> Result<Vec<MeetingPerson>, AppError> {
    attendance.attendees(&meeting_id).await.into_ipc()
}

/// Adds a person to the meeting as an invitee.
#[tauri::command]
#[specta::specta]
async fn add_attendee(
    attendance: State<'_, Arc<AttendanceService>>,
    meeting_id: MeetingId,
    person_id: PersonId,
) -> Result<(), AppError> {
    attendance.add(&meeting_id, &person_id).await.into_ipc()
}

/// Takes a person off the meeting; `true` when they were on it.
#[tauri::command]
#[specta::specta]
async fn remove_attendee(
    attendance: State<'_, Arc<AttendanceService>>,
    meeting_id: MeetingId,
    person_id: PersonId,
) -> Result<bool, AppError> {
    attendance.remove(&meeting_id, &person_id).await.into_ipc()
}
