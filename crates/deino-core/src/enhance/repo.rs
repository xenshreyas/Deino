//! The enhanced-notes ports and their SQLite adapter over `enhanced_notes`.
//!
//! [`EnhancedNotesReader`] is the stable read API other features use
//! (exports, EXP-01; search, ORG-08; MCP, DEV-02); [`EnhancedNotesWriter`]
//! is the service's alone. The document is parsed into `deino_doc::Doc` at
//! this edge, so the rest of the crate never sees JSON.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_doc::Doc;
use deino_domain::{MeetingId, SeriesId, TemplateId};
use sqlx::types::Json;

use super::material::PreviousNotes;
use super::{EnhanceBasis, EnhancedNotes, NewVersion, VersionInfo};

/// An edit of the newest version.
#[derive(Clone, Debug, PartialEq)]
pub struct VersionEdit {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The version edited; it must be the newest.
    pub version: u32,
    /// The edited document, normalized and valid.
    pub doc: Doc,
    /// Its Readable Markdown.
    pub markdown: String,
    /// When it was saved.
    pub at: DateTime<Utc>,
}

/// Reads meetings' enhanced notes.
#[async_trait]
pub trait EnhancedNotesReader: Send + Sync {
    /// The newest version of the meeting's notes, if any.
    async fn latest(&self, meeting: &MeetingId) -> Result<Option<EnhancedNotes>, DbError>;

    /// One version of the meeting's notes.
    async fn version(
        &self,
        meeting: &MeetingId,
        version: u32,
    ) -> Result<Option<EnhancedNotes>, DbError>;

    /// Every version of the meeting's notes, newest first.
    async fn versions(&self, meeting: &MeetingId) -> Result<Vec<VersionInfo>, DbError>;

    /// The newest notes of the latest other meeting of `series` before
    /// `before`, outside the trash.
    async fn previous_in_series(
        &self,
        series: &SeriesId,
        meeting: &MeetingId,
        before: DateTime<Utc>,
    ) -> Result<Option<PreviousNotes>, DbError>;
}

/// Writes meetings' enhanced notes.
#[async_trait]
pub trait EnhancedNotesWriter: Send + Sync {
    /// Stores `version` as the meeting's next version; returns its number.
    async fn insert(&self, version: &NewVersion) -> Result<u32, DbError>;

    /// Saves an edit of the newest version and marks it user-edited.
    /// Returns `false`, changing nothing, when `edit.version` is not the
    /// newest version.
    async fn save_edit(&self, edit: &VersionEdit) -> Result<bool, DbError>;
}

/// The enhanced-notes ports over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteEnhancedNotesRepo {
    pub(super) database: Database,
}

impl SqliteEnhancedNotesRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl EnhancedNotesReader for SqliteEnhancedNotesRepo {
    async fn latest(&self, meeting: &MeetingId) -> Result<Option<EnhancedNotes>, DbError> {
        let row = sqlx::query_file_as!(VersionRow, "src/enhance/queries/latest.sql", meeting)
            .fetch_optional(self.database.reader())
            .await?;
        row.map(EnhancedNotes::try_from).transpose()
    }

    async fn version(
        &self,
        meeting: &MeetingId,
        version: u32,
    ) -> Result<Option<EnhancedNotes>, DbError> {
        let row = sqlx::query_file_as!(
            VersionRow,
            "src/enhance/queries/version.sql",
            meeting,
            version
        )
        .fetch_optional(self.database.reader())
        .await?;
        row.map(EnhancedNotes::try_from).transpose()
    }

    async fn versions(&self, meeting: &MeetingId) -> Result<Vec<VersionInfo>, DbError> {
        let versions =
            sqlx::query_file_as!(VersionInfo, "src/enhance/queries/versions.sql", meeting)
                .fetch_all(self.database.reader())
                .await?;
        Ok(versions)
    }

    async fn previous_in_series(
        &self,
        series: &SeriesId,
        meeting: &MeetingId,
        before: DateTime<Utc>,
    ) -> Result<Option<PreviousNotes>, DbError> {
        let previous = sqlx::query_file_as!(
            PreviousNotes,
            "src/enhance/queries/previous_in_series.sql",
            series,
            meeting,
            before
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(previous)
    }
}

#[async_trait]
impl EnhancedNotesWriter for SqliteEnhancedNotesRepo {
    async fn insert(&self, version: &NewVersion) -> Result<u32, DbError> {
        let doc = serde_json::to_string(&version.doc).map_err(decode_error)?;
        let basis = version.basis.as_ref().map(Json);
        let mut transaction = self.database.writer().begin().await?;
        let number = sqlx::query_scalar!(
            r#"SELECT COALESCE(MAX(version), 0) + 1 AS "next!: u32"
               FROM enhanced_notes WHERE meeting_id = ?1"#,
            version.meeting_id,
        )
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query!(
            "INSERT INTO enhanced_notes (meeting_id, version, doc_json, markdown, template_id,
                model, basis_json, is_user_edited, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            version.meeting_id,
            number,
            doc,
            version.markdown,
            version.template_id,
            version.model,
            basis,
            version.is_user_edited,
            version.at,
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(number)
    }

    async fn save_edit(&self, edit: &VersionEdit) -> Result<bool, DbError> {
        let doc = serde_json::to_string(&edit.doc).map_err(decode_error)?;
        let saved = sqlx::query!(
            "UPDATE enhanced_notes
             SET doc_json = ?3, markdown = ?4, is_user_edited = 1, updated_at = ?5
             WHERE meeting_id = ?1 AND version = ?2
                AND version = (SELECT MAX(version) FROM enhanced_notes WHERE meeting_id = ?1)",
            edit.meeting_id,
            edit.version,
            doc,
            edit.markdown,
            edit.at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(saved.rows_affected() > 0)
    }
}

/// One `enhanced_notes` row as stored.
struct VersionRow {
    meeting_id: MeetingId,
    version: u32,
    doc_json: String,
    markdown: String,
    template_id: Option<TemplateId>,
    model: String,
    basis: Option<Json<EnhanceBasis>>,
    is_user_edited: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<VersionRow> for EnhancedNotes {
    type Error = DbError;

    fn try_from(row: VersionRow) -> Result<Self, DbError> {
        Ok(Self {
            doc: serde_json::from_str(&row.doc_json).map_err(decode_error)?,
            meeting_id: row.meeting_id,
            version: row.version,
            markdown: row.markdown,
            template_id: row.template_id,
            model: row.model,
            basis: row.basis.map(|basis| basis.0),
            is_user_edited: row.is_user_edited,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// A stored document that does not parse is a decode failure of the row.
fn decode_error(error: impl std::error::Error + Send + Sync + 'static) -> DbError {
    DbError::Query(sqlx::Error::Decode(Box::new(error)))
}
