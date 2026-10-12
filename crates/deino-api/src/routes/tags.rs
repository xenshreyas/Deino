//! `POST /v1/tags`, `PUT /v1/tags/{id}` and `DELETE /v1/tags/{id}`
//! through the app's `TagService` (`GET` is in `directory.rs`).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use deino_core::query::TagView;
use deino_core::tags::{NewTag, Tag, TagUpdate};
use deino_domain::TagId;
use serde::Deserialize;

use crate::error::{ApiResult, app};
use crate::extract::{Body, Id};
use crate::{ApiError, ApiState};

/// `POST /v1/tags`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TagInput {
    /// Its name; unique, ignoring case (409 when taken).
    pub name: String,
    /// When it applies, for the auto-tagger.
    #[serde(default)]
    pub description: String,
}

/// Makes a tag.
pub async fn create(
    State(state): State<ApiState>,
    Body(input): Body<TagInput>,
) -> ApiResult<(StatusCode, Json<TagView>)> {
    let new = NewTag {
        name: input.name,
        color: None,
        description: input.description,
    };
    let tag = state.services.writes.tags.create(&new).await.map_err(app)?;
    Ok((StatusCode::CREATED, Json(view(&state, tag).await?)))
}

/// `PUT /v1/tags/{id}`: each field present replaces its value.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TagChange {
    /// A new name.
    pub name: Option<String>,
    /// A new description.
    pub description: Option<String>,
}

/// Renames a tag or changes its description.
pub async fn update(
    State(state): State<ApiState>,
    Id(id): Id<TagId>,
    Body(change): Body<TagChange>,
) -> ApiResult<Json<TagView>> {
    let tags = &state.services.writes.tags;
    let current = tags
        .list()
        .await
        .map_err(app)?
        .into_iter()
        .find(|tag| tag.id == id)
        .ok_or_else(|| ApiError::not_found(format!("tag {id} was not found")))?;
    let update = TagUpdate {
        name: change.name.unwrap_or(current.name),
        color: current.color,
        description: change.description.unwrap_or(current.description),
        auto_apply: current.auto_apply,
    };
    let tag = tags.update(&id, &update).await.map_err(app)?;
    Ok(Json(view(&state, tag).await?))
}

/// Deletes a tag and takes it off every meeting.
pub async fn delete(State(state): State<ApiState>, Id(id): Id<TagId>) -> ApiResult<StatusCode> {
    state.services.writes.tags.delete(&id).await.map_err(app)?;
    Ok(StatusCode::NO_CONTENT)
}

/// The tag as `GET /v1/tags` lists it.
async fn view(state: &ApiState, tag: Tag) -> ApiResult<TagView> {
    let tags = state
        .services
        .library
        .directory
        .tags()
        .await
        .map_err(app)?;
    tags.into_iter()
        .find(|view| view.id == tag.id)
        .ok_or_else(|| ApiError::not_found(format!("tag {} was not found", tag.id)))
}
