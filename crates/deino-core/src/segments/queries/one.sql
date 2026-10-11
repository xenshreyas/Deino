-- One segment by ID (?1), deleted or not: the transcript edits read it to
-- learn its meeting and current text, and to restore a deleted one.
SELECT
    id AS "id: SegmentId",
    meeting_id AS "meeting_id: MeetingId",
    channel AS "channel: Channel",
    speaker_id AS "speaker_id: SpeakerId",
    start_ms AS "start: Millis",
    end_ms AS "end: Millis",
    text,
    words_json AS "words: Json<Vec<WordTiming>>",
    is_deleted AS "is_deleted: bool",
    edited_at AS "edited_at: DateTime<Utc>",
    original_text
FROM transcript_segments
WHERE id = ?1
