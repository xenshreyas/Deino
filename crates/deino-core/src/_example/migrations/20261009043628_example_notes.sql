-- The `_example` reference feature's notes (debug builds, in-memory only).
-- New tables are STRICT, NOT NULL by default and validate in CHECK constraints
-- (PARITY_PLAN §3A); IDs are TEXT UUIDv7 and timestamps TEXT RFC 3339 UTC.
CREATE TABLE example_notes (
    id TEXT PRIMARY KEY NOT NULL,
    text TEXT NOT NULL CHECK (length(trim(text)) > 0),
    created_at TEXT NOT NULL
) STRICT;

-- Every list screen's sort order gets an index (asserted in repo.rs tests).
CREATE INDEX idx_example_notes_created_at_id ON example_notes (created_at, id);
