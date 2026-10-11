-- Action items; ?1 is one meeting's ID, or NULL for every item (standalone
-- ones included). The same rule as the `search_action_items_*` triggers.
INSERT INTO search_documents (kind, ref_id, meeting_id, title, body)
SELECT 'action_item', id, meeting_id, title, description FROM action_items
WHERE ?1 IS NULL OR meeting_id = ?1
