-- Segments find and replace (TRN-02) may change: not deleted, in a meeting
-- outside the trash, in one meeting (?1) or all (?1 IS NULL), whose text is
-- LIKE ?2 (`%needle%`, `\` escapes; case-insensitive for ASCII, so a
-- superset of a case-sensitive match that the caller narrows). Newest
-- meeting first, then spoken order. One meeting uses
-- idx_transcript_segments_meeting_id_start_ms; all meetings scan the table,
-- which a deliberate, rare bulk edit can afford.
SELECT
    s.id AS "id: SegmentId",
    s.meeting_id AS "meeting_id: MeetingId",
    m.title AS "meeting_title",
    s.text
FROM transcript_segments AS s
JOIN meetings AS m ON m.id = s.meeting_id
WHERE s.is_deleted = 0
    AND m.deleted_at IS NULL
    AND (?1 IS NULL OR s.meeting_id = ?1)
    AND s.text LIKE ?2 ESCAPE '\'
ORDER BY coalesce(m.started_at, m.created_at) DESC, s.meeting_id, s.start_ms, s.id
