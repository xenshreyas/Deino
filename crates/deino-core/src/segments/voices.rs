//! SQL for diarization's writes: speakers with their voices, and segments
//! re-attributed to them.
//!
//! A centroid is stored as little-endian `f32`s in `speakers.centroid`
//! (`20261009075316_cap03_speaker_voices.sql`).

use deino_db::{Database, DbError};
use deino_domain::{MeetingId, Millis, SegmentId, SpeakerId};
use sqlx::SqliteConnection;
use sqlx::types::Json;

use super::{NewSpeaker, Reattribution, SegmentPiece, SpeakerVoice};

/// Replaces the meeting's speakers other than the user's in one transaction.
pub(super) async fn replace(
    database: &Database,
    meeting: &MeetingId,
    speakers: &[NewSpeaker],
) -> Result<(), DbError> {
    let mut transaction = database.writer().begin().await?;
    sqlx::query!(
        "DELETE FROM speakers WHERE meeting_id = ?1 AND is_me = 0",
        meeting
    )
    .execute(&mut *transaction)
    .await?;
    for new in speakers {
        insert_speaker(&mut transaction, new).await?;
    }
    transaction.commit().await?;
    Ok(())
}

async fn insert_speaker(
    connection: &mut SqliteConnection,
    new: &NewSpeaker,
) -> Result<(), DbError> {
    let speaker = &new.speaker;
    let centroid = new.voice.as_ref().map(|voice| encode(&voice.centroid));
    let model = new
        .voice
        .as_ref()
        .map(|voice| voice.embedding_model.as_str());
    let speech = new.voice.as_ref().map(|voice| voice.speech);
    sqlx::query!(
        "INSERT INTO speakers (id, meeting_id, label, person_id, is_me, color, centroid, embedding_model, speech_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        speaker.id,
        speaker.meeting_id,
        speaker.label,
        speaker.person_id,
        speaker.is_me,
        speaker.color,
        centroid,
        model,
        speech,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// The speaker's stored voice, if it has one.
pub(super) async fn voice(
    database: &Database,
    id: &SpeakerId,
) -> Result<Option<SpeakerVoice>, DbError> {
    let row = sqlx::query!(
        r#"SELECT centroid, embedding_model, speech_ms AS "speech_ms: Millis"
        FROM speakers WHERE id = ?1"#,
        id
    )
    .fetch_optional(database.reader())
    .await?;
    let voice = row.and_then(|row| {
        Some(SpeakerVoice {
            centroid: decode(&row.centroid?),
            embedding_model: row.embedding_model?,
            speech: row.speech_ms?,
        })
    });
    Ok(voice)
}

/// Applies every re-attribution in one transaction.
pub(super) async fn reattribute(
    database: &Database,
    changes: &[Reattribution],
) -> Result<(), DbError> {
    let mut transaction = database.writer().begin().await?;
    for change in changes {
        update_piece(&mut transaction, &change.id, &change.head).await?;
        for (id, piece) in &change.tail {
            insert_piece(&mut transaction, &change.id, id, piece).await?;
        }
    }
    transaction.commit().await?;
    Ok(())
}

/// Rewrites a segment as its first piece; `edited_at` is left alone.
async fn update_piece(
    connection: &mut SqliteConnection,
    id: &SegmentId,
    piece: &SegmentPiece,
) -> Result<(), DbError> {
    let words = Json(&piece.words);
    sqlx::query!(
        "UPDATE transcript_segments
         SET speaker_id = ?1, start_ms = ?2, end_ms = ?3, text = ?4, words_json = ?5
         WHERE id = ?6",
        piece.speaker_id,
        piece.start,
        piece.end,
        piece.text,
        words,
        id,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Adds a piece split off `original`, on its meeting and channel.
async fn insert_piece(
    connection: &mut SqliteConnection,
    original: &SegmentId,
    id: &SegmentId,
    piece: &SegmentPiece,
) -> Result<(), DbError> {
    let words = Json(&piece.words);
    sqlx::query!(
        "INSERT INTO transcript_segments (id, meeting_id, channel, speaker_id, start_ms, end_ms, text, words_json)
         SELECT ?1, meeting_id, channel, ?2, ?3, ?4, ?5, ?6 FROM transcript_segments WHERE id = ?7",
        id,
        piece.speaker_id,
        piece.start,
        piece.end,
        piece.text,
        words,
        original,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// `values` as little-endian bytes.
pub(super) fn encode(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// Little-endian bytes back into values (a trailing partial value is
/// dropped; the schema allows none).
pub(super) fn decode(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .filter_map(|chunk| chunk.try_into().ok().map(f32::from_le_bytes))
        .collect()
}
