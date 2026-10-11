-- AI-05: saved views of the Action Items page (ADR 0028).
--
-- A view is a named filter the page shows as a tab. The filter is a typed
-- Rust value (`deino_core::action_items::ActionItemFilter`) stored as JSON:
-- the page reads a view whole and the list query takes the parsed filter, so
-- no SQL ever reads a field of it (PARITY_PLAN §3A "JSON columns").
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, timestamps RFC 3339
-- UTC, an index for the list order.

CREATE TABLE action_item_views (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    name TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK (length(trim(name)) > 0),
    filter_json TEXT NOT NULL
        CHECK (json_valid(filter_json) AND json_type(filter_json) = 'object'),
    -- 0-based order of the tabs; `SqliteActionItemViewRepo::create` appends.
    position INTEGER NOT NULL CHECK (position >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- The tabs, in order.
CREATE INDEX idx_action_item_views_position ON action_item_views (position, id);
