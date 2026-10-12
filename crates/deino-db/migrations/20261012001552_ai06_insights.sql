-- AI-06: custom AI insights (ADR 0036).
--
-- An insight is a question the user asks of meetings: a name, a description
-- that is the prompt, optional typed fields, how many answers to keep, and
-- when it runs on its own. `insight_results` holds each insight's latest run
-- on each meeting.
--
-- JSON columns (§3A): `fields_json` (`Vec<InsightField>`) and
-- `conditions_json` (`deino_domain::ConditionGroups`) are parts of one
-- definition, read and written whole and never queried by field.
-- `instances_json` (`Vec<StoredInstance>`) is a write-once snapshot of one
-- run's validated answers, replaced whole by the next run; each answer cites
-- a transcript segment by ID, with no foreign key, like enhanced notes'
-- citations: a citation of a segment deleted later stops resolving, and the
-- stored speaker name and time remain.
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, timestamps RFC 3339
-- UTC, an index for every foreign key and list order.

CREATE TABLE insight_defs (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
        CHECK (length(trim(name)) BETWEEN 1 AND 80),
    -- What to extract or write: the prompt.
    description TEXT NOT NULL CHECK (length(trim(description)) > 0),
    fields_json TEXT NOT NULL
        CHECK (json_valid(fields_json) AND json_type(fields_json) = 'array'),
    -- NULL: the model decides how many answers (Auto); else at most this many.
    max_answers INTEGER CHECK (max_answers IS NULL OR max_answers BETWEEN 1 AND 20),
    -- `automatic` runs after every meeting its conditions match (no
    -- conditions: every meeting); `manual` runs only when asked.
    apply_mode TEXT NOT NULL CHECK (apply_mode IN ('automatic', 'manual')),
    conditions_json TEXT NOT NULL
        CHECK (json_valid(conditions_json) AND json_type(conditions_json) = 'array'),
    -- Bumped by every edit of what the insight asks (name, description,
    -- fields, count), so a result can say it answers an older question.
    revision INTEGER NOT NULL CHECK (revision >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- The Insights page lists definitions oldest first.
CREATE INDEX idx_insight_defs_created ON insight_defs (created_at, id);

CREATE TABLE insight_results (
    insight_id TEXT NOT NULL REFERENCES insight_defs (id) ON DELETE CASCADE,
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    -- `pending`: asked for and waiting for the `insights` pipeline stage;
    -- `done`: `instances_json` answers revision `revision`; `failed`: the
    -- last run failed with `error` (a user-safe sentence), earlier answers kept.
    status TEXT NOT NULL CHECK (status IN ('pending', 'done', 'failed')),
    instances_json TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(instances_json) AND json_type(instances_json) = 'array'),
    -- The definition's revision the answers were written for; NULL until the
    -- first answer.
    revision INTEGER CHECK (revision IS NULL OR revision >= 1),
    error TEXT CHECK ((status = 'failed') = (error IS NOT NULL)),
    requested_at TEXT NOT NULL,
    generated_at TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (insight_id, meeting_id)
) STRICT;

-- A meeting's Insights tab, and its cascade.
CREATE INDEX idx_insight_results_meeting ON insight_results (meeting_id);
