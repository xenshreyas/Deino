-- FND-13: index the meetings list's sort order.
-- The sidebar lists meetings newest first (`ORDER BY created_at DESC, id DESC`).
-- Without this index SQLite scans the table and sorts it in a temporary B-tree;
-- with it, the list is a backwards index scan. `id` breaks ties deterministically.
CREATE INDEX idx_meetings_created_at_id ON meetings (created_at, id);
