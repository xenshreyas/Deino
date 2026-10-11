-- Enhanced notes (NOT-02): the newest version of each meeting's AI notes, as
-- its `summary` document; ?1 is one meeting's ID, or NULL. The same rule as
-- the `search_enhanced_notes_*` triggers.
INSERT INTO search_documents (kind, ref_id, meeting_id, body)
SELECT 'summary', meeting_id, meeting_id, markdown FROM enhanced_notes AS e
WHERE (?1 IS NULL OR meeting_id = ?1)
    AND version = (SELECT max(version) FROM enhanced_notes WHERE meeting_id = e.meeting_id)
    AND trim(markdown) <> ''
