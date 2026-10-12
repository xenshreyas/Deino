//! `POST /v1/meetings` (import a transcript), `PUT /v1/meetings/{id}` and
//! `DELETE /v1/meetings/{id}` (to the trash): each goes through the app's
//! own service, never SQL here.

use std::collections::HashSet;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use deino_core::query::MeetingDetail;
use deino_core::transcript_import::TranscriptImport;
use deino_domain::{MeetingId, TagId, TagSource};
use serde::Deserialize;

use crate::error::{ApiResult, app};
use crate::extract::Body;
use crate::{ApiError, ApiState};

/// Imports a transcript (Circleback's import shape) as a new meeting.
pub async fn create(
    State(state): State<ApiState>,
    Body(import): Body<TranscriptImport>,
) -> ApiResult<(StatusCode, Json<MeetingDetail>)> {
    let services = &state.services;
    let id = services
        .writes
        .importer
        .import(&import)
        .await
        .map_err(app)?;
    let detail = services.library.meetings.read(&id).await.map_err(app)?;
    Ok((StatusCode::CREATED, Json(detail)))
}

/// `PUT /v1/meetings/{id}`: each field present replaces its value.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeetingUpdate {
    /// A new title.
    pub title: Option<String>,
    /// The user's own notes, as Markdown.
    pub my_notes: Option<String>,
    /// Exactly these tags, by ID.
    pub tag_ids: Option<Vec<TagId>>,
}

/// Changes a meeting's title, notes or tags.
pub async fn update(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Body(update): Body<MeetingUpdate>,
) -> ApiResult<Json<MeetingDetail>> {
    let id = MeetingId::new(id);
    let services = &state.services;
    services.library.meetings.read(&id).await.map_err(app)?;
    if let Some(title) = &update.title {
        services
            .writes
            .meetings
            .rename(&id, title)
            .await
            .map_err(app)?;
    }
    if let Some(markdown) = &update.my_notes {
        let doc = deino_doc::from_markdown(markdown)
            .map_err(|_| ApiError::invalid("myNotes is not Markdown Deino can read"))?;
        services.writes.notes.save(&id, doc).await.map_err(app)?;
    }
    if let Some(tags) = &update.tag_ids {
        replace_tags(&state, &id, tags).await?;
    }
    Ok(Json(services.library.meetings.read(&id).await.map_err(app)?))
}

/// Leaves exactly `wanted` on the meeting.
async fn replace_tags(state: &ApiState, id: &MeetingId, wanted: &[TagId]) -> ApiResult<()> {
    let tags = &state.services.writes.tags;
    let wanted: HashSet<TagId> = wanted.iter().copied().collect();
    let current = tags.tags_of(id).await.map_err(app)?;
    for applied in &current {
        if !wanted.contains(&applied.tag.id) {
            tags.remove(id, &applied.tag.id).await.map_err(app)?;
        }
    }
    for tag in &wanted {
        tags.apply(id, tag, TagSource::User).await.map_err(app)?;
    }
    Ok(())
}

/// Moves a meeting to the trash (ADR 0046); it can be restored in the app
/// for 30 days.
pub async fn delete(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let id = MeetingId::new(id);
    let outcome = state
        .services
        .writes
        .trash
        .trash(std::slice::from_ref(&id))
        .await
        .map_err(app)?;
    if outcome.changed.is_empty() {
        return Err(ApiError::not_found(format!("meeting {id} was not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}
