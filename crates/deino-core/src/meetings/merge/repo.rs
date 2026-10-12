//! [`MeetingMergeRepo`]: reading merge candidates and applying a
//! [`MergePlan`] in one transaction, over SQLite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{MeetingId, MeetingStatus, Millis};
use sqlx::types::Json;

use super::{MergeCandidate, MergePlan, steps};
use crate::meetings::{MeetingAudio, SqliteMeetingRepo};
use crate::notes::upsert_notes;

/// Where merges read and write.
#[async_trait]
pub trait MeetingMergeRepo: Send + Sync {
    /// The meetings among `ids` that exist, in no particular order.
    async fn candidates(&self, ids: &[MeetingId]) -> Result<Vec<MergeCandidate>, DbError>;

    /// Applies `plan` in one transaction: all of it, or nothing.
    async fn apply_merge(&self, plan: &MergePlan) -> Result<(), DbError>;
}

/// One `candidates.sql` row.
struct CandidateRow {
    id: MeetingId,
    title: String,
    held_at: DateTime<Utc>,
    status: MeetingStatus,
    trashed: bool,
    length: i64,
    parts: i64,
    audio_path: Option<String>,
    audio_mic_path: Option<String>,
    audio_system_path: Option<String>,
    video_path: Option<String>,
}

#[async_trait]
impl MeetingMergeRepo for SqliteMeetingRepo {
    async fn candidates(&self, ids: &[MeetingId]) -> Result<Vec<MergeCandidate>, DbError> {
        let ids = Json(ids);
        let rows = sqlx::query_file_as!(
            CandidateRow,
            "src/meetings/merge/queries/candidates.sql",
            ids
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(CandidateRow::into_candidate).collect())
    }

    async fn apply_merge(&self, plan: &MergePlan) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for part in &plan.parts {
            steps::record_part(&mut transaction, plan, part).await?;
            steps::move_speakers(&mut transaction, &plan.target, part).await?;
            steps::move_transcript(&mut transaction, &plan.target, part).await?;
            steps::move_links(&mut transaction, &plan.target, &part.source).await?;
            steps::delete_source(&mut transaction, &part.source).await?;
        }
        if let Some(notes) = &plan.notes {
            upsert_notes(&mut transaction, notes).await?;
        }
        steps::finish(&mut transaction, plan).await?;
        transaction.commit().await?;
        Ok(())
    }
}

impl CandidateRow {
    fn into_candidate(self) -> MergeCandidate {
        MergeCandidate {
            id: self.id,
            title: self.title,
            held_at: self.held_at,
            status: self.status,
            trashed: self.trashed,
            length: Millis(u32::try_from(self.length).unwrap_or(u32::MAX)),
            parts: u32::try_from(self.parts).unwrap_or(0),
            audio: MeetingAudio {
                audio_path: self.audio_path,
                audio_mic_path: self.audio_mic_path,
                audio_system_path: self.audio_system_path,
                video_path: self.video_path,
            },
        }
    }
}
