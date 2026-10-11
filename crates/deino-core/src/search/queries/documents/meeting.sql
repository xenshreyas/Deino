-- Meeting titles; ?1 is one meeting's ID, or NULL for every meeting.
-- The same rule as the `search_meetings_*` triggers (migration org08).
INSERT INTO search_documents (kind, ref_id, meeting_id, title)
SELECT 'meeting', id, id, title FROM meetings
WHERE ?1 IS NULL OR id = ?1
