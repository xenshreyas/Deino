-- AUT-01: automations (ADR 0042).
--
-- An automation is a name, conditions ("when to run": the AI-06 condition
-- groups) and an ordered list of steps (generate an insight, tag, file,
-- export, notify, run a shell script, ...). `automation_runs` holds each
-- automation's latest run on each meeting, and `automation_run_steps` each
-- step of that run with its own status, so a retry resumes after the last
-- step that finished and the run log shows every step's result and error.
--
-- JSON columns (§3A): `conditions_json` (`deino_domain::ConditionGroups`)
-- and `steps_json` (`Vec<deino_core::automations::Action>`) are parts of one
-- definition, read and written whole and never queried by field.
-- `output_json` (`StepOutput`) is a write-once snapshot of what one step
-- produced, read back whole by later steps of the same run.
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, timestamps RFC 3339
-- UTC, an index for every foreign key and list order.

CREATE TABLE automations (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
        CHECK (length(trim(name)) BETWEEN 1 AND 80),
    -- Off: never runs on its own, but can still be run by hand.
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    -- Runs after every matching meeting is processed (and when a tag its
    -- conditions test for is applied); off: manual runs only.
    auto_run INTEGER NOT NULL CHECK (auto_run IN (0, 1)),
    -- A failed step skips the steps after it; off: they still run.
    stop_on_failure INTEGER NOT NULL CHECK (stop_on_failure IN (0, 1)),
    conditions_json TEXT NOT NULL
        CHECK (json_valid(conditions_json) AND json_type(conditions_json) = 'array'),
    steps_json TEXT NOT NULL
        CHECK (json_valid(steps_json) AND json_type(steps_json) = 'array'),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- The Automations page lists them oldest first.
CREATE INDEX idx_automations_created ON automations (created_at, id);

CREATE TABLE automation_runs (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    automation_id TEXT NOT NULL REFERENCES automations (id) ON DELETE CASCADE,
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    trigger TEXT NOT NULL CHECK (trigger IN ('pipeline', 'tag_applied', 'manual')),
    -- The tag whose application started a `tag_applied` run.
    trigger_tag_id TEXT REFERENCES tags (id) ON DELETE SET NULL,
    -- `armed`: a tag armed it on a meeting still to come; `pending`: waiting
    -- for the `automations` stage; `done`: every step ran or was skipped;
    -- `failed`: a step failed (see its row).
    status TEXT NOT NULL CHECK (status IN ('armed', 'pending', 'done', 'failed')),
    requested_at TEXT NOT NULL,
    finished_at TEXT,
    updated_at TEXT NOT NULL,
    -- One latest run per automation and meeting: a rerun replaces it.
    UNIQUE (automation_id, meeting_id)
) STRICT;

-- A meeting's ⋯ → Automations menu, and its cascade.
CREATE INDEX idx_automation_runs_meeting ON automation_runs (meeting_id);
-- The tag foreign key's SET NULL.
CREATE INDEX idx_automation_runs_trigger_tag ON automation_runs (trigger_tag_id)
    WHERE trigger_tag_id IS NOT NULL;

CREATE TABLE automation_run_steps (
    run_id TEXT NOT NULL REFERENCES automation_runs (id) ON DELETE CASCADE,
    -- The step's index in the automation's `steps_json` when the run started.
    position INTEGER NOT NULL CHECK (position >= 0),
    -- The `Action` kind, so the log reads even after the automation changes.
    kind TEXT NOT NULL CHECK (length(kind) > 0),
    status TEXT NOT NULL CHECK (status IN ('pending', 'done', 'failed', 'skipped')),
    -- A failed step's `AutomationError` code and user-safe sentence.
    error_code TEXT CHECK ((status = 'failed') = (error_code IS NOT NULL)),
    error TEXT CHECK ((status = 'failed') = (error IS NOT NULL)),
    output_json TEXT CHECK (output_json IS NULL OR json_valid(output_json)),
    finished_at TEXT,
    PRIMARY KEY (run_id, position)
) STRICT;
