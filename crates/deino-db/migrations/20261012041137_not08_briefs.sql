-- NOT-08: pre-meeting briefs (ADR 0047).
--
-- One row per upcoming calendar event the brief job looked at. `ready` rows
-- hold 2–4 cited bullets; `insufficient` rows record that there was too
-- little to say, so the same inputs never reach the model twice.
--
-- `inputs_hash` is the SHA-256 (hex) of the rendered prompt the brief was
-- written from: the job regenerates exactly when it changes, which makes it
-- idempotent across ticks and restarts.
--
-- JSON column (§3A): `bullets_json` (`Vec<BriefBullet>`) is a write-once
-- snapshot of one generation, read and replaced whole, never queried by
-- field. Each bullet's citations name meetings, segments, people and
-- companies by ID without foreign keys, like enhanced notes' citations: a
-- citation of something deleted later stops resolving and keeps its label.
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, timestamps RFC 3339
-- UTC. The primary key is the only lookup.

CREATE TABLE briefs (
    event_id TEXT PRIMARY KEY NOT NULL
        REFERENCES calendar_events (id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('ready', 'insufficient')),
    inputs_hash TEXT NOT NULL CHECK (length(inputs_hash) = 64),
    bullets_json TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(bullets_json) AND json_type(bullets_json) = 'array'),
    generated_at TEXT NOT NULL,
    -- When the user first saw the brief; the card collapses from then on.
    -- Cleared when the brief is regenerated.
    read_at TEXT,
    -- The thumbs down; cleared when the brief is regenerated.
    feedback TEXT CHECK (feedback IS NULL OR feedback = 'not_useful'),
    feedback_at TEXT CHECK ((feedback IS NULL) = (feedback_at IS NULL)),
    CHECK ((status = 'ready') = (json_array_length(bullets_json) > 0))
) STRICT;
