//! [`AskStore`]: a meeting's thread in the `ask_messages` table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{AskMessageId, MeetingId};
use sqlx::types::Json;

use super::{AnswerPart, AskMessage, AskRole, RecipeLabel};

/// The most messages a thread keeps; older ones are pruned (a short
/// history, D13).
pub const MAX_THREAD_MESSAGES: i64 = 40;

/// Reads and writes meetings' threads.
#[async_trait]
pub trait AskStore: Send + Sync {
    /// The meeting's thread, oldest first.
    async fn thread(&self, meeting: &MeetingId) -> Result<Vec<AskMessage>, DbError>;

    /// Appends a question and its answer, then prunes the thread to
    /// [`MAX_THREAD_MESSAGES`].
    async fn append(&self, question: &AskMessage, answer: &AskMessage) -> Result<(), DbError>;

    /// Empties the meeting's thread.
    async fn clear(&self, meeting: &MeetingId) -> Result<(), DbError>;
}

/// [`AskStore`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteAskRepo {
    database: Database,
}

impl SqliteAskRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// One `ask_messages` row.
struct Row {
    id: AskMessageId,
    meeting_id: MeetingId,
    role: AskRole,
    parts: Json<Vec<AnswerPart>>,
    recipe_slug: Option<String>,
    recipe_name: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<Row> for AskMessage {
    fn from(row: Row) -> Self {
        let recipe = row
            .recipe_slug
            .zip(row.recipe_name)
            .map(|(slug, name)| RecipeLabel { slug, name });
        Self {
            id: row.id,
            meeting_id: row.meeting_id,
            role: row.role,
            parts: row.parts.0,
            recipe,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl AskStore for SqliteAskRepo {
    async fn thread(&self, meeting: &MeetingId) -> Result<Vec<AskMessage>, DbError> {
        let rows = sqlx::query_as!(
            Row,
            r#"SELECT id AS "id!: AskMessageId", meeting_id AS "meeting_id!: MeetingId",
                role AS "role!: AskRole", parts_json AS "parts!: Json<Vec<AnswerPart>>",
                recipe_slug, recipe_name, created_at AS "created_at!: DateTime<Utc>"
            FROM ask_messages WHERE meeting_id = ?1 ORDER BY created_at, id"#,
            meeting,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(AskMessage::from).collect())
    }

    async fn append(&self, question: &AskMessage, answer: &AskMessage) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for message in [question, answer] {
            let parts = Json(&message.parts);
            let (slug, name) = message
                .recipe
                .as_ref()
                .map(|recipe| (recipe.slug.as_str(), recipe.name.as_str()))
                .unzip();
            sqlx::query!(
                "INSERT INTO ask_messages
                    (id, meeting_id, role, parts_json, recipe_slug, recipe_name, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                message.id,
                message.meeting_id,
                message.role,
                parts,
                slug,
                name,
                message.created_at,
            )
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query!(
            "DELETE FROM ask_messages WHERE meeting_id = ?1 AND id NOT IN (
                 SELECT id FROM ask_messages WHERE meeting_id = ?1
                 ORDER BY created_at DESC, id DESC LIMIT ?2)",
            question.meeting_id,
            MAX_THREAD_MESSAGES,
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn clear(&self, meeting: &MeetingId) -> Result<(), DbError> {
        sqlx::query!("DELETE FROM ask_messages WHERE meeting_id = ?1", meeting)
            .execute(self.database.writer())
            .await?;
        Ok(())
    }
}
