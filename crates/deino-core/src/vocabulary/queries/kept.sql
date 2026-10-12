-- The kept originals that are not undone, beside each segment's text now:
-- every one of meeting ?1, or segment ?2's (the other parameter is NULL), in
-- transcript order. A meeting's segments are found through
-- idx_transcript_segments_meeting_id_start_ms (asserted in corrections_tests.rs).
SELECT
    c.segment_id AS "segment_id: SegmentId",
    c.original_text,
    c.original_words_json AS "original_words: Json<Vec<WordTiming>>",
    c.corrected_text,
    s.text
FROM transcript_segments AS s
JOIN vocab_corrections AS c ON c.segment_id = s.id
WHERE c.undone_at IS NULL AND (s.meeting_id = ?1 OR s.id = ?2)
ORDER BY s.start_ms, s.id
