//! `GET /v1/meetings`, `GET /v1/meetings/{id}` and
//! `GET /v1/meetings/{id}/transcript`: reads through `deino-core::query`,
//! the CLI's read layer, so each returns what `deino --json` returns.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::Uri;
use chrono::NaiveDate;
use deino_core::query::{
    MAX_LINES, MeetingCursor, MeetingDetail, MeetingFilter, MeetingSummary, TranscriptLine,
    TranscriptWindow,
};
use deino_domain::{MeetingId, Millis};
use serde::{Deserialize, Serialize};

use crate::error::{ApiResult, app};
use crate::extract::Params;
use crate::page::{self, Page, Paged};
use crate::{ApiError, ApiState};

/// The most transcript lines `include=transcript` inlines; longer ones are
/// read with `/transcript`.
const MAX_INLINE_LINES: usize = 20_000;

/// `GET /v1/meetings`: filters, newest first.
#[derive(Debug, Default, Deserialize, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeetingParams {
    /// The first local day, inclusive (`YYYY-MM-DD`).
    pub from: Option<NaiveDate>,
    /// The last local day, inclusive.
    pub to: Option<NaiveDate>,
    /// A tag's ID or name.
    pub tag: Option<String>,
    /// A folder's ID or name (its subfolders too).
    pub folder: Option<String>,
    /// A person's ID, name or email.
    pub person: Option<String>,
    /// A company's ID, name or domain.
    pub company: Option<String>,
    /// Page size, 1 to 100 (default 20).
    pub limit: Option<u32>,
    /// The `nextCursor` of the previous page.
    pub cursor: Option<String>,
}

impl MeetingParams {
    /// The filters alone, for the cursor's fingerprint.
    fn filters(&self) -> u64 {
        let key = (&self.from, &self.to, &self.tag, &self.folder, &self.person);
        page::fingerprint(&(key, &self.company, page::page_size(self.limit)))
    }
}

/// Lists meetings a page at a time.
pub async fn list(
    State(state): State<ApiState>,
    uri: Uri,
    Params(params): Params<MeetingParams>,
) -> ApiResult<Paged<MeetingSummary>> {
    let filters = params.filters();
    let after: Option<MeetingCursor> = params
        .cursor
        .as_deref()
        .map(|cursor| page::decode(cursor, filters))
        .transpose()?;
    let filter = MeetingFilter {
        limit: page::page_size(params.limit),
        from: params.from,
        to: params.to,
        tag: params.tag,
        folder: params.folder,
        person: params.person,
        company: params.company,
    };
    let found = state
        .services
        .library
        .meetings
        .page(&filter, after.as_ref())
        .await
        .map_err(app)?;
    let page = Page {
        data: found.meetings,
        next_cursor: found.next.map(|next| page::encode(&next, filters)),
    };
    Ok(Paged { page, uri })
}

/// `GET /v1/meetings/{id}?include=transcript`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadParams {
    /// `transcript` inlines the transcript.
    pub include: Option<String>,
}

/// A meeting, with its transcript when asked for.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingResponse {
    /// Everything but the transcript.
    #[serde(flatten)]
    pub detail: MeetingDetail,
    /// The transcript, with `include=transcript`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<Vec<TranscriptLine>>,
}

/// Reads one meeting.
pub async fn read(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Params(params): Params<ReadParams>,
) -> ApiResult<Json<MeetingResponse>> {
    let id = MeetingId::new(id);
    let transcript = match params.include.as_deref() {
        None | Some("") => None,
        Some("transcript") => Some(whole_transcript(&state, &id).await?),
        Some(_) => return Err(ApiError::invalid("include may only be `transcript`")),
    };
    let detail = state
        .services
        .library
        .meetings
        .read(&id)
        .await
        .map_err(app)?;
    Ok(Json(MeetingResponse { detail, transcript }))
}

/// Every line of the transcript, up to [`MAX_INLINE_LINES`].
async fn whole_transcript(state: &ApiState, id: &MeetingId) -> ApiResult<Vec<TranscriptLine>> {
    let mut lines = Vec::new();
    let mut from = None;
    loop {
        let window = TranscriptWindow {
            from,
            to: None,
            limit: MAX_LINES,
        };
        let page = state
            .services
            .library
            .transcripts
            .page(id, window)
            .await
            .map_err(app)?;
        lines.extend(page.lines);
        if lines.len() > MAX_INLINE_LINES {
            return Err(ApiError::invalid(
                "the transcript is too long to inline; page it with /transcript",
            ));
        }
        match page.next_from_ms {
            Some(next) => from = Some(Millis(next)),
            None => return Ok(lines),
        }
    }
}

/// `GET /v1/meetings/{id}/transcript`: a page of lines.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TranscriptParams {
    /// Page size, 1 to 2000 (default 300).
    pub limit: Option<u32>,
    /// The `nextCursor` of the previous page.
    pub cursor: Option<String>,
}

/// Reads a transcript a page at a time.
pub async fn transcript(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    uri: Uri,
    Params(params): Params<TranscriptParams>,
) -> ApiResult<Paged<TranscriptLine>> {
    let id = MeetingId::new(id);
    let filters = page::fingerprint(&(id.as_str(), params.limit));
    let from: Option<u32> = params
        .cursor
        .as_deref()
        .map(|cursor| page::decode(cursor, filters))
        .transpose()?;
    let window = TranscriptWindow {
        from: from.map(Millis),
        to: None,
        limit: params.limit.unwrap_or(0),
    };
    let found = state
        .services
        .library
        .transcripts
        .page(&id, window)
        .await
        .map_err(app)?;
    let page = Page {
        data: found.lines,
        next_cursor: found.next_from_ms.map(|next| page::encode(&next, filters)),
    };
    Ok(Paged { page, uri })
}
