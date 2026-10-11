//! The semantic index's ports: what a meeting says, and the stored chunks.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, Millis};

use super::{Chunk, ChunkKind, StoredVector, TranscriptLine};
use crate::search::SearchError;

/// Everything of a meeting worth embedding.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeetingSources {
    /// The transcript, in time order, without deleted segments.
    pub transcript: Vec<TranscriptLine>,
    /// The user's notes as Readable Markdown, if any.
    pub notes: Option<String>,
    /// The summary as Markdown, if any.
    pub summary: Option<String>,
}

/// A chunk embedded earlier, so an unchanged one is not embedded again.
#[derive(Clone, Debug, PartialEq)]
pub struct ExistingEmbedding {
    /// Its text.
    pub text: String,
    /// The model that embedded it.
    pub model: String,
    /// Its vector.
    pub vector: Vec<f32>,
}

/// A stored chunk with its meeting, for a search result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkRow {
    /// The `embeddings` row.
    pub id: i64,
    /// Its meeting.
    pub meeting_id: MeetingId,
    /// What it was cut from.
    pub kind: ChunkKind,
    /// The window's first segment, or the meeting for a section.
    pub ref_id: String,
    /// The window's start in the recording.
    pub ts_ms: Option<Millis>,
    /// The window's end.
    pub end_ms: Option<Millis>,
    /// Its text.
    pub text: String,
    /// The meeting's title.
    pub title: String,
    /// When the meeting happened.
    pub date: DateTime<Utc>,
}

/// A meeting's new chunks, to replace its old ones.
#[derive(Clone, Copy, Debug)]
pub struct Replacement<'a> {
    /// The meeting.
    pub meeting: &'a MeetingId,
    /// The model that embedded the chunks.
    pub model: &'a str,
    /// The chunks, in order.
    pub chunks: &'a [Chunk],
    /// One vector per chunk, in the same order.
    pub vectors: &'a [Vec<f32>],
    /// When they were embedded.
    pub at: DateTime<Utc>,
}

/// How much of the meetings with something to embed a model covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    /// Meetings with chunks embedded by the model.
    pub indexed: u32,
    /// Meetings outside the trash with a transcript, notes or a summary.
    pub total: u32,
}

/// Reads what meetings say.
#[async_trait]
pub trait MeetingSourceReader: Send + Sync {
    /// The meeting's transcript, notes and summary.
    async fn sources(&self, meeting: &MeetingId) -> Result<MeetingSources, SearchError>;

    /// Finished meetings outside the trash with something to embed, newest
    /// first: the backfill's work list.
    async fn to_index(&self) -> Result<Vec<MeetingId>, SearchError>;
}

/// The stored chunks and their vectors (`embeddings`).
#[async_trait]
pub trait EmbeddingStore: Send + Sync {
    /// The meeting's chunks as stored now.
    async fn existing(&self, meeting: &MeetingId) -> Result<Vec<ExistingEmbedding>, SearchError>;

    /// Replaces the meeting's chunks with the replacement's, in one
    /// transaction, and returns the new rows for the in-memory index.
    async fn replace(&self, replacement: Replacement<'_>)
    -> Result<Vec<StoredVector>, SearchError>;

    /// Every vector embedded by `model`, for the in-memory index.
    async fn load(&self, model: &str) -> Result<Vec<StoredVector>, SearchError>;

    /// The rows with these IDs whose meetings are not in the trash.
    async fn hydrate(&self, ids: &[i64]) -> Result<Vec<ChunkRow>, SearchError>;

    /// How much `model` covers.
    async fn coverage(&self, model: &str) -> Result<Coverage, SearchError>;
}
