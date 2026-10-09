-- FND-08: the post-meeting pipeline's durable queue (PARITY_PLAN §4.2,
-- ADR 0018). Conventions as in the FND-07 core schema: STRICT, NOT NULL by
-- default, enums CHECKed against their `deino_domain` enum, timestamps as
-- RFC 3339 UTC text, an index on every foreign key and lookup.
--
-- One `jobs` row per (meeting, stage) holds that stage's state in the
-- meeting's current pipeline run; scheduling the pipeline again resets the
-- row. `job_runs` keeps one row per attempt: the history, and the open rows
-- that startup reconciliation closes after a crash.
--
-- Stage IDs are names from the code (`legacy_summary`), not rows of a table:
-- stages are registered in `crates/deino-pipeline/src/registry.rs`, and
-- startup deletes the jobs of a stage that is no longer registered.
-- Nothing here is a denormalised copy.

CREATE TABLE jobs (
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    stage TEXT NOT NULL CHECK (length(stage) > 0),
    status TEXT NOT NULL
        CHECK (status IN ('pending', 'running', 'succeeded', 'skipped', 'failed', 'cancelled')),
    -- Attempts started in the current run; back to 0 when the job is scheduled again.
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    -- A pending retry waits until this time (backoff); NULL runs as soon as it is next.
    not_before TEXT CHECK (not_before IS NULL OR status = 'pending'),
    -- Why the stage was skipped or how its last attempt failed. User-safe
    -- text from the stage, never meeting content.
    message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (meeting_id, stage)
) STRICT;

-- Startup lists the meetings that still have pending or running jobs.
CREATE INDEX idx_jobs_status_meeting_id ON jobs (status, meeting_id);

CREATE TABLE job_runs (
    -- History rows are never referenced, so the rowid is their key.
    id INTEGER PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    attempt INTEGER NOT NULL CHECK (attempt >= 1),
    started_at TEXT NOT NULL,
    -- Both NULL while the attempt runs; a crash leaves them NULL until startup.
    finished_at TEXT CHECK (finished_at IS NULL OR julianday(finished_at) >= julianday(started_at)),
    outcome TEXT CHECK (outcome IN ('succeeded', 'skipped', 'failed', 'cancelled', 'interrupted')),
    message TEXT,
    CHECK ((finished_at IS NULL) = (outcome IS NULL)),
    FOREIGN KEY (meeting_id, stage) REFERENCES jobs (meeting_id, stage) ON DELETE CASCADE
) STRICT;

-- A job's attempts in order; also serves the foreign key.
CREATE INDEX idx_job_runs_meeting_id_stage_id ON job_runs (meeting_id, stage, id);
