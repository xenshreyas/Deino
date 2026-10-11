-- A meeting's (?1) transcript in time order, without deleted segments,
-- windowed by LIMIT ?2 OFFSET ?3. Served by
-- idx_transcript_segments_meeting_id_start_ms (asserted in repo_tests.rs).
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
WHERE meeting_id = ?1 AND is_deleted = 0
ORDER BY start_ms, id
LIMIT ?2 OFFSET ?3
