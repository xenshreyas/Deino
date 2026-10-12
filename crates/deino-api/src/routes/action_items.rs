//! `GET/POST /v1/action-items` and `PUT/DELETE /v1/action-items/{id}`:
//! reads through the CLI's `ActionItemQueries`, writes through the app's
//! `ActionItemService` (so the Action Items page updates at once).

use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, Uri};
use chrono::NaiveDate;
use deino_core::action_items::{ActionItemEdit, ActionItemView, NewActionItem, StatusFilter};
use deino_core::query::{ActionItemSummary, Assignee, ItemFilter, MAX_ITEMS, summarize_item};
use deino_domain::{ActionItemId, ActionItemPriority, ActionItemStatus, MeetingId, PersonId};
use serde::Deserialize;

use crate::error::{ApiResult, app};
use crate::extract::{Body, Id, Params};
use crate::page::{self, Paged};
use crate::{ApiError, ApiState};

/// `GET /v1/action-items` parameters.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemParams {
    /// `open` (default), `done` or `any`.
    pub status: Option<StatusFilter>,
    /// `anyone` (default), `me`, `others`, `unassigned`, or a person's ID,
    /// name or email.
    pub assignee: Option<String>,
    /// Only this meeting's.
    pub meeting_id: Option<String>,
    /// Page size, 1 to 100 (default 20).
    pub limit: Option<u32>,
    /// The `nextCursor` of the previous page.
    pub cursor: Option<String>,
}

fn assignee(text: Option<&str>) -> Assignee {
    match text {
        None | Some("anyone") => Assignee::Anyone,
        Some("me") => Assignee::Me,
        Some("others") => Assignee::Others,
        Some("unassigned") => Assignee::Unassigned,
        Some(person) => Assignee::Person(person.to_owned()),
    }
}

/// Lists action items, dated ones soonest first.
pub async fn list(
    State(state): State<ApiState>,
    uri: Uri,
    Params(params): Params<ItemParams>,
) -> ApiResult<Paged<ActionItemSummary>> {
    let size = page::page_size(params.limit);
    let status = format!("{:?}", params.status);
    let filters = page::fingerprint(&(status, &params.assignee, &params.meeting_id, size));
    let filter = ItemFilter {
        status: params.status.unwrap_or_default(),
        assignee: assignee(params.assignee.as_deref()),
        due: None,
        meeting: params.meeting_id.map(MeetingId::new),
        limit: MAX_ITEMS,
    };
    let items = state
        .services
        .library
        .action_items
        .list(&filter)
        .await
        .map_err(app)?;
    let page = page::by_position(items, params.cursor.as_deref(), (size, filters))?;
    Ok(Paged { page, uri })
}

/// `POST /v1/action-items`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemInput {
    /// What to do.
    pub title: String,
    /// Context.
    #[serde(default)]
    pub description: String,
    /// The meeting it belongs to; none for a standalone to-do.
    pub meeting_id: Option<String>,
    /// Who should do it.
    pub assignee_person_id: Option<PersonId>,
    /// The local day it is due.
    pub due: Option<NaiveDate>,
    /// `low`, `medium` or `high`.
    pub priority: Option<ActionItemPriority>,
}

/// Adds an action item.
pub async fn create(
    State(state): State<ApiState>,
    Body(input): Body<ItemInput>,
) -> ApiResult<(StatusCode, Json<ActionItemSummary>)> {
    let new = NewActionItem {
        title: input.title,
        description: input.description,
        meeting_id: input.meeting_id.map(MeetingId::new),
        assignee_person_id: input.assignee_person_id,
        due: input.due,
        priority: input.priority,
    };
    let services = &state.services;
    let view = services
        .writes
        .action_items
        .create(new)
        .await
        .map_err(app)?;
    Ok((StatusCode::CREATED, Json(summary(&state, view))))
}

/// `PUT /v1/action-items/{id}`: each field present replaces its value.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemChange {
    /// A new title.
    pub title: Option<String>,
    /// A new description.
    pub description: Option<String>,
    /// `open` or `done`.
    pub status: Option<ActionItemStatus>,
    /// A new due day.
    pub due: Option<NaiveDate>,
    /// A new assignee.
    pub assignee_person_id: Option<PersonId>,
}

impl ItemChange {
    fn edits(self) -> Vec<ActionItemEdit> {
        let edits = [
            self.title.map(|title| ActionItemEdit::Title { title }),
            self.description
                .map(|description| ActionItemEdit::Description { description }),
            self.status.map(|status| ActionItemEdit::Status { status }),
            self.due.map(|date| ActionItemEdit::Due { date: Some(date) }),
            self.assignee_person_id.map(|person| ActionItemEdit::Assignee {
                person_id: Some(person),
            }),
        ];
        edits.into_iter().flatten().collect()
    }
}

/// Changes an action item.
pub async fn update(
    State(state): State<ApiState>,
    Id(id): Id<ActionItemId>,
    Body(change): Body<ItemChange>,
) -> ApiResult<Json<ActionItemSummary>> {
    let edits = change.edits();
    if edits.is_empty() {
        return Err(ApiError::invalid("send at least one field to change"));
    }
    let items = &state.services.writes.action_items;
    let mut view = None;
    for edit in &edits {
        view = Some(items.update(id, edit).await.map_err(app)?);
    }
    let view = view.ok_or_else(|| ApiError::invalid("nothing changed"))?;
    Ok(Json(summary(&state, view)))
}

/// Deletes an action item.
pub async fn delete(
    State(state): State<ApiState>,
    Id(id): Id<ActionItemId>,
) -> ApiResult<StatusCode> {
    let deleted = state
        .services
        .writes
        .action_items
        .delete(&[id])
        .await
        .map_err(app)?;
    if deleted == 0 {
        return Err(ApiError::not_found(format!("action item {id} was not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

fn summary(state: &ApiState, view: ActionItemView) -> ActionItemSummary {
    summarize_item(view, state.services.zone.as_ref())
}
