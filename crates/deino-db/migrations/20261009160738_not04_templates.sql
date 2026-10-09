-- NOT-04: note templates move from JSON files into the database (ADR 0021).
--
-- A template is a row in `templates` plus its ordered `template_sections`.
-- Built-ins carry the catalogue version they were last written from
-- (`builtin_version`); `deino_core::templates::BuiltinSync` inserts and
-- upgrades them at startup and never touches the user's own templates, whose
-- `builtin_version` is NULL. Built-ins are immutable: "Duplicate" makes an
-- editable copy with a new ID.
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, enums CHECK-ed
-- against their `deino_domain` enum, timestamps RFC 3339 UTC, an index for
-- every foreign key and list order.

CREATE TABLE templates (
    -- A readable slug for built-ins (`one_on_one`), a UUIDv7 for the user's own.
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) BETWEEN 1 AND 64 AND id = trim(id)),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    -- One line: what the template is for. The picker shows it and the LLM
    -- classifier chooses by it.
    description TEXT NOT NULL DEFAULT '',
    -- Who the meeting is with and what the notes are for, given to the LLM.
    meeting_context TEXT NOT NULL DEFAULT '',
    -- When to choose the template automatically (`Vec<AutoRule>` in Rust).
    -- Opaque to SQL: the rule engine evaluates it in Rust (title patterns are
    -- regular expressions), so no query ever reads a field of it.
    auto_rules_json TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(auto_rules_json) AND json_type(auto_rules_json) = 'array'),
    -- NULL for the user's templates; for built-ins, the catalogue version the
    -- row was written from (0 for a placeholder this migration carried over).
    builtin_version INTEGER CHECK (builtin_version IS NULL OR builtin_version >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- The template list and pickers sort by name.
CREATE INDEX idx_templates_name ON templates (name COLLATE NOCASE, id);

-- Sections are their own rows rather than a JSON column so the schema keeps
-- their invariants (a non-blank title, a known format, a gapless order per
-- template through the primary key) and a template's sections are replaced
-- in the same transaction as its row.
CREATE TABLE template_sections (
    template_id TEXT NOT NULL REFERENCES templates (id) ON DELETE CASCADE,
    -- 0-based order within the template.
    position INTEGER NOT NULL CHECK (position >= 0),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    instructions TEXT NOT NULL DEFAULT '',
    format TEXT NOT NULL CHECK (format IN ('bullets', 'paragraph', 'table', 'todo')),
    PRIMARY KEY (template_id, position)
) STRICT;

-- `meetings.template_id` becomes a real reference (PARITY_PLAN §7.2, FND-07).
-- SQLite cannot add a constraint to an existing column, and rebuilding
-- `meetings` would cascade-delete its legacy children, so the column is
-- renamed, re-added with the reference, copied and dropped.
--
-- Until now it could only name a legacy built-in JSON template. Those six map
-- onto their NOT-04 successors; the placeholder rows inserted here
-- (`builtin_version = 0`) are filled in by `BuiltinSync` at the next start.
-- Any other value never named a template the app could load, so it is cleared.
ALTER TABLE meetings RENAME COLUMN template_id TO legacy_template_id;
ALTER TABLE meetings ADD COLUMN template_id TEXT REFERENCES templates (id) ON DELETE SET NULL;

CREATE TEMP TABLE legacy_template_ids (legacy_id TEXT PRIMARY KEY, id TEXT NOT NULL);
INSERT INTO legacy_template_ids (legacy_id, id) VALUES
    ('standard_meeting', 'general'),
    ('daily_standup', 'standup'),
    ('project_sync', 'weekly_team_sync'),
    ('retrospective', 'retrospective'),
    ('sales_marketing_client_call', 'customer_check_in'),
    ('psychatric_session', 'therapy_soap');

INSERT INTO templates (id, name, builtin_version, created_at, updated_at)
SELECT DISTINCT map.id, map.id, 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
FROM meetings
JOIN legacy_template_ids AS map ON map.legacy_id = meetings.legacy_template_id;

UPDATE meetings
SET template_id = (SELECT id FROM legacy_template_ids WHERE legacy_id = meetings.legacy_template_id)
WHERE legacy_template_id IS NOT NULL;

DROP TABLE legacy_template_ids;
ALTER TABLE meetings DROP COLUMN legacy_template_id;

-- The foreign key, and "which meetings use this template" before a delete.
CREATE INDEX idx_meetings_template_id ON meetings (template_id) WHERE template_id IS NOT NULL;
