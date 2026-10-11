-- Transcript segments that are not soft-deleted; ?1 is one meeting's ID, or
-- NULL. The same rule as the `search_segments_*` triggers.
INSERT INTO search_documents (kind, ref_id, meeting_id, body, ts_ms)
SELECT 'segment', id, meeting_id, text, start_ms FROM transcript_segments
WHERE (?1 IS NULL OR meeting_id = ?1) AND is_deleted = 0 AND trim(text) <> ''
