-- The user's notes (Readable Markdown); ?1 is one meeting's ID, or NULL.
-- The same rule as the `search_meeting_notes_*` triggers.
INSERT INTO search_documents (kind, ref_id, meeting_id, body)
SELECT 'note', meeting_id, meeting_id, notes_markdown FROM meeting_notes
WHERE (?1 IS NULL OR meeting_id = ?1) AND trim(coalesce(notes_markdown, '')) <> ''
