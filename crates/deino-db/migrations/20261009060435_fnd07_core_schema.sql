-- FND-07: the core schema (PARITY_PLAN §4.1), in one migration so later cards
-- add columns and tables around it instead of colliding.
--
-- Conventions for every table created here (§3A "Database design"):
-- - STRICT, NOT NULL unless a value can genuinely be unknown, enums checked
--   with CHECK (each maps to a `deino_domain` enum with the same snake_case
--   names), booleans as INTEGER 0/1.
-- - IDs are TEXT UUIDs (UUIDv7 for new rows), 36 characters, checked.
--   Timestamps are TEXT RFC 3339 UTC, as sqlx encodes `DateTime<Utc>`.
-- - Real foreign keys with an explicit ON DELETE, and an index on every
--   foreign key and every list filter or sort (asserted with EXPLAIN QUERY
--   PLAN in the repository tests in `deino-core`).
-- - No WITHOUT ROWID tables: SQLite's update hook, which drives `DbChanged`,
--   does not fire for them.
-- - JSON columns hold opaque payloads only, never queried by field.
--
-- Denormalised columns, each with its source of truth and the one place that
-- keeps it in sync:
-- - `meetings.duration_s`: see the column comment below.
-- Nothing else here is a copy: links live in exactly one place (for example
-- the meeting <-> calendar event link is `meetings.calendar_event_id` only; see
-- ADR 0012 for why `calendar_events` has no `meeting_id`).

-- Companies ------------------------------------------------------------------

CREATE TABLE companies (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    -- The company's web domain ("acme.com"), lower case; NULL for companies
    -- added by hand without one. Unique, so people upsert onto one company.
    domain TEXT UNIQUE CHECK (
        domain IS NULL OR (length(domain) > 0 AND domain = lower(trim(domain)) AND domain NOT LIKE '%@%')
    ),
    logo_path TEXT,
    notes_md TEXT NOT NULL DEFAULT '',
    summary_md TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX idx_companies_name ON companies (name COLLATE NOCASE, id);

-- People ---------------------------------------------------------------------

CREATE TABLE people (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    -- Lower case, so the UNIQUE constraint is case-insensitive.
    email TEXT UNIQUE CHECK (email IS NULL OR (email = lower(trim(email)) AND email LIKE '_%@_%')),
    company_id TEXT REFERENCES companies (id) ON DELETE SET NULL,
    title TEXT,
    avatar_path TEXT,
    notes_md TEXT NOT NULL DEFAULT '',
    summary_md TEXT,
    summary_updated_at TEXT,
    is_me INTEGER NOT NULL DEFAULT 0 CHECK (is_me IN (0, 1)),
    -- Set when this person was merged into another; merged people are hidden
    -- from lists. Clearing the target un-merges rather than deleting.
    merged_into TEXT REFERENCES people (id) ON DELETE SET NULL CHECK (merged_into IS NULL OR merged_into <> id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX idx_people_company_id ON people (company_id);
-- Partial, so the people list never mistakes it for a way to find unmerged
-- people; it still serves the foreign key (`merged_into = ?` implies NOT NULL).
CREATE INDEX idx_people_merged_into ON people (merged_into) WHERE merged_into IS NOT NULL;
-- The people list: unmerged people by name.
CREATE INDEX idx_people_listed_name ON people (name COLLATE NOCASE, id) WHERE merged_into IS NULL;
-- At most one person is the user.
CREATE UNIQUE INDEX idx_people_is_me ON people (is_me) WHERE is_me = 1;

-- Calendars ------------------------------------------------------------------

CREATE TABLE calendar_accounts (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    provider TEXT NOT NULL CHECK (provider IN ('apple', 'google', 'microsoft')),
    -- What Settings shows: the account's email, or "On My Mac" for EventKit.
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    created_at TEXT NOT NULL,
    UNIQUE (provider, name)
) STRICT;

CREATE TABLE calendar_events (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    account_id TEXT NOT NULL REFERENCES calendar_accounts (id) ON DELETE CASCADE,
    provider_event_id TEXT NOT NULL CHECK (length(provider_event_id) > 0),
    ical_uid TEXT,
    -- The provider's recurring-series key; NULL for one-off events.
    series_id TEXT,
    title TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    start_at TEXT NOT NULL,
    end_at TEXT NOT NULL CHECK (julianday(end_at) >= julianday(start_at)),
    conference_url TEXT,
    platform TEXT CHECK (platform IN ('zoom', 'google_meet', 'teams', 'webex', 'slack', 'other')),
    organizer_email TEXT,
    -- The provider's attendee list as received (`Vec<EventAttendee>` in Rust).
    -- Opaque: the queryable copy is `meeting_attendees`, filled when a meeting
    -- is linked to the event.
    attendees_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(attendees_json) AND json_type(attendees_json) = 'array'),
    my_rsvp TEXT CHECK (my_rsvp IN ('accepted', 'declined', 'tentative', 'needs_action')),
    is_external INTEGER NOT NULL DEFAULT 0 CHECK (is_external IN (0, 1)),
    -- NULL follows the auto-record rules; 1 always records, 0 never does.
    record_override INTEGER CHECK (record_override IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (account_id, provider_event_id)
) STRICT;

-- "Coming up" and the event picker list events by start time.
CREATE INDEX idx_calendar_events_start_at ON calendar_events (start_at, id);
CREATE INDEX idx_calendar_events_series_id ON calendar_events (series_id) WHERE series_id IS NOT NULL;

-- Meetings (extended) --------------------------------------------------------
-- `meetings` predates STRICT and is referenced by legacy tables, so it is
-- extended in place: rebuilding it would cascade-delete every child row.

ALTER TABLE meetings ADD COLUMN status TEXT NOT NULL DEFAULT 'ready'
    CHECK (status IN ('scheduled', 'recording', 'processing', 'ready', 'failed'));
-- Unknown (NULL) for meetings saved before FND-07, which only kept `created_at`.
ALTER TABLE meetings ADD COLUMN started_at TEXT;
ALTER TABLE meetings ADD COLUMN ended_at TEXT
    CHECK (ended_at IS NULL OR started_at IS NULL OR julianday(ended_at) >= julianday(started_at));
-- Denormalised: the recorded length in whole seconds, excluding pauses, so
-- lists never measure audio files. Source of truth: the meeting's audio
-- tracks. Kept in sync by `MeetingLifecycleRepo::set_timing` only (the
-- recorder calls it when it stops); backfilled below for older meetings from
-- the end of their last transcript segment.
ALTER TABLE meetings ADD COLUMN duration_s INTEGER CHECK (duration_s >= 0);
ALTER TABLE meetings ADD COLUMN calendar_event_id TEXT REFERENCES calendar_events (id) ON DELETE SET NULL;
-- The recurring-series key copied from the linked event when it is linked,
-- and kept when the event later leaves the sync window.
ALTER TABLE meetings ADD COLUMN series_id TEXT;
-- The note template; NOT-04 moves templates into the database and turns
-- this into a reference. Until then it names a built-in template.
ALTER TABLE meetings ADD COLUMN template_id TEXT;
ALTER TABLE meetings ADD COLUMN source TEXT NOT NULL DEFAULT 'live'
    CHECK (source IN ('live', 'import', 'api'));
-- Set when the meeting is moved to the trash.
ALTER TABLE meetings ADD COLUMN deleted_at TEXT;
-- The mixed recording and the per-channel tracks (FND-05); NULL when absent.
ALTER TABLE meetings ADD COLUMN audio_path TEXT;
ALTER TABLE meetings ADD COLUMN audio_mic_path TEXT;
ALTER TABLE meetings ADD COLUMN audio_system_path TEXT;
ALTER TABLE meetings ADD COLUMN video_path TEXT;

-- The meetings list hides the trash; this partial index replaces FND-13's
-- full one, which the list no longer uses.
DROP INDEX idx_meetings_created_at_id;
CREATE INDEX idx_meetings_live_created_at_id ON meetings (created_at, id) WHERE deleted_at IS NULL;
CREATE INDEX idx_meetings_trashed_deleted_at ON meetings (deleted_at, id) WHERE deleted_at IS NOT NULL;
-- Recovery finds meetings stuck in `recording`; the pipeline finds `processing`.
CREATE INDEX idx_meetings_status_created_at ON meetings (status, created_at, id);
CREATE INDEX idx_meetings_calendar_event_id ON meetings (calendar_event_id) WHERE calendar_event_id IS NOT NULL;
CREATE INDEX idx_meetings_series_id ON meetings (series_id) WHERE series_id IS NOT NULL;

-- Speakers -------------------------------------------------------------------

CREATE TABLE speakers (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    -- What the transcript shows until a person is assigned ("Speaker 2").
    label TEXT NOT NULL CHECK (length(trim(label)) > 0),
    person_id TEXT REFERENCES people (id) ON DELETE SET NULL,
    is_me INTEGER NOT NULL DEFAULT 0 CHECK (is_me IN (0, 1)),
    -- A slot of the shared `--speaker-N` palette (docs/references/ui.md).
    color INTEGER NOT NULL CHECK (color BETWEEN 1 AND 6),
    UNIQUE (meeting_id, label)
) STRICT;

CREATE INDEX idx_speakers_person_id ON speakers (person_id);

-- Transcript segments --------------------------------------------------------

CREATE TABLE transcript_segments (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    channel TEXT NOT NULL CHECK (channel IN ('mic', 'system', 'mixed')),
    speaker_id TEXT REFERENCES speakers (id) ON DELETE SET NULL,
    -- Milliseconds from the start of the recording.
    start_ms INTEGER NOT NULL CHECK (start_ms >= 0),
    end_ms INTEGER NOT NULL CHECK (end_ms >= start_ms),
    text TEXT NOT NULL,
    -- Token timestamps (`Vec<WordTiming>` in Rust); `[]` when the engine has none.
    words_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(words_json) AND json_type(words_json) = 'array'),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    edited_at TEXT
) STRICT;

-- The transcript reads a meeting's segments in time order (and the FK).
CREATE INDEX idx_transcript_segments_meeting_id_start_ms ON transcript_segments (meeting_id, start_ms, id);
CREATE INDEX idx_transcript_segments_speaker_id ON transcript_segments (speaker_id);

-- A segment's speaker must belong to the segment's meeting. A composite
-- foreign key cannot express this together with ON DELETE SET NULL.
CREATE TRIGGER transcript_segments_speaker_in_meeting_on_insert
BEFORE INSERT ON transcript_segments
WHEN NEW.speaker_id IS NOT NULL
    AND (SELECT meeting_id FROM speakers WHERE id = NEW.speaker_id) IS NOT NEW.meeting_id
BEGIN
    SELECT RAISE(ABORT, 'a segment''s speaker must belong to the same meeting');
END;

CREATE TRIGGER transcript_segments_speaker_in_meeting_on_update
BEFORE UPDATE OF speaker_id, meeting_id ON transcript_segments
WHEN NEW.speaker_id IS NOT NULL
    AND (SELECT meeting_id FROM speakers WHERE id = NEW.speaker_id) IS NOT NEW.meeting_id
BEGIN
    SELECT RAISE(ABORT, 'a segment''s speaker must belong to the same meeting');
END;

-- Move the legacy transcript into `transcript_segments`. Legacy capture mixed
-- the microphone and system audio before transcription, so every row is
-- `mixed`. IDs were `transcript-<UUIDv4>`; the UUID is kept, and anything else
-- gets a fresh random UUID. Rows without audio timing (saved before Oct 2025)
-- start at 0. Orphans of a deleted meeting were invisible and are dropped.
INSERT INTO transcript_segments (id, meeting_id, channel, start_ms, end_ms, text)
SELECT
    CASE
        WHEN substr(t.id, -36) GLOB '????????-????-????-????-????????????' THEN lower(substr(t.id, -36))
        ELSE lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
            || substr(lower(hex(randomblob(2))), 2) || '-'
            || substr('89ab', 1 + abs(random()) % 4, 1) || substr(lower(hex(randomblob(2))), 2) || '-'
            || lower(hex(randomblob(6)))
    END,
    t.meeting_id,
    'mixed',
    max(0, coalesce(CAST(round(t.audio_start_time * 1000) AS INTEGER), 0)),
    max(
        0,
        coalesce(CAST(round(t.audio_start_time * 1000) AS INTEGER), 0),
        coalesce(
            CAST(round(t.audio_end_time * 1000) AS INTEGER),
            CAST(round((t.audio_start_time + t.duration) * 1000) AS INTEGER),
            0
        )
    ),
    t.transcript
FROM transcripts AS t
WHERE t.meeting_id IN (SELECT id FROM meetings)
ORDER BY t.rowid;

UPDATE meetings
SET duration_s = (
    SELECT (max(s.end_ms) + 500) / 1000 FROM transcript_segments AS s WHERE s.meeting_id = meetings.id
);

DROP TABLE transcripts;

-- Compatibility view (ADR 0012): the inherited summary, import,
-- re-transcription and meeting-page code still reads and writes the legacy
-- `transcripts` shape. Drop the view and its triggers once FND-05b, CAP-11,
-- CAP-08 and TRN-01 have ported the last of them to the segment repository.
-- `timestamp` used to be a wall-clock string; it is now the recording offset
-- (HH:MM:SS), which is what the UI falls back to anyway.
CREATE VIEW transcripts AS
SELECT
    s.id AS id,
    s.meeting_id AS meeting_id,
    s.text AS transcript,
    printf('%02d:%02d:%02d', s.start_ms / 3600000, s.start_ms / 60000 % 60, s.start_ms / 1000 % 60) AS timestamp,
    NULL AS summary,
    NULL AS action_items,
    NULL AS key_points,
    s.start_ms / 1000.0 AS audio_start_time,
    s.end_ms / 1000.0 AS audio_end_time,
    (s.end_ms - s.start_ms) / 1000.0 AS duration,
    NULL AS speaker
FROM transcript_segments AS s
WHERE s.is_deleted = 0;

CREATE TRIGGER transcripts_insert
INSTEAD OF INSERT ON transcripts
BEGIN
    INSERT INTO transcript_segments (id, meeting_id, channel, start_ms, end_ms, text)
    VALUES (
        CASE
            WHEN substr(NEW.id, -36) GLOB '????????-????-????-????-????????????' THEN lower(substr(NEW.id, -36))
            ELSE lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
                || substr(lower(hex(randomblob(2))), 2) || '-'
                || substr('89ab', 1 + abs(random()) % 4, 1) || substr(lower(hex(randomblob(2))), 2) || '-'
                || lower(hex(randomblob(6)))
        END,
        NEW.meeting_id,
        'mixed',
        max(0, coalesce(CAST(round(NEW.audio_start_time * 1000) AS INTEGER), 0)),
        max(
            0,
            coalesce(CAST(round(NEW.audio_start_time * 1000) AS INTEGER), 0),
            coalesce(
                CAST(round(NEW.audio_end_time * 1000) AS INTEGER),
                CAST(round((NEW.audio_start_time + NEW.duration) * 1000) AS INTEGER),
                0
            )
        ),
        NEW.transcript
    );
END;

CREATE TRIGGER transcripts_delete
INSTEAD OF DELETE ON transcripts
BEGIN
    DELETE FROM transcript_segments WHERE id = OLD.id;
END;

-- Attendees ------------------------------------------------------------------

CREATE TABLE meeting_attendees (
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people (id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('organizer', 'invitee', 'speaker_only')),
    -- NULL when the person was not invited (`speaker_only`) or the calendar
    -- did not say.
    rsvp TEXT CHECK (rsvp IN ('accepted', 'declined', 'tentative', 'needs_action')),
    PRIMARY KEY (meeting_id, person_id)
) STRICT;

-- The FK, and a person's meetings without touching the table.
CREATE INDEX idx_meeting_attendees_person_id ON meeting_attendees (person_id, meeting_id);

-- Folders --------------------------------------------------------------------

CREATE TABLE folders (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- Deleting a folder deletes its subfolders; meetings are never deleted
    -- with a folder, they only leave it.
    parent_id TEXT REFERENCES folders (id) ON DELETE CASCADE CHECK (parent_id IS NULL OR parent_id <> id),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    -- An icon name from the UI's icon set; NULL shows the default folder icon.
    icon TEXT,
    -- Context for chat and auto-filing.
    description TEXT NOT NULL DEFAULT '',
    -- Auto-add rules; ORG-05 defines the payload and its Rust type.
    auto_rules_json TEXT CHECK (auto_rules_json IS NULL OR json_valid(auto_rules_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- Sibling names are unique; root folders share the parent ''.
CREATE UNIQUE INDEX idx_folders_parent_id_name ON folders (coalesce(parent_id, ''), name COLLATE NOCASE);
-- The FK, and the tree lists children by name.
CREATE INDEX idx_folders_parent_id ON folders (parent_id, name COLLATE NOCASE);

-- A folder cannot move under one of its own descendants. SQLite allows no
-- recursive query inside a trigger, so `FolderRepo::move_to` checks it in
-- the same transaction as the move; that is the only writer of `parent_id`.

CREATE TABLE meeting_folders (
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    folder_id TEXT NOT NULL REFERENCES folders (id) ON DELETE CASCADE,
    added_at TEXT NOT NULL,
    PRIMARY KEY (meeting_id, folder_id)
) STRICT;

CREATE INDEX idx_meeting_folders_folder_id ON meeting_folders (folder_id, meeting_id);

-- Tags -----------------------------------------------------------------------

CREATE TABLE tags (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    name TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK (length(trim(name)) > 0),
    -- A slot of the shared `--speaker-N` palette (tag colours reuse it).
    color INTEGER NOT NULL CHECK (color BETWEEN 1 AND 6),
    -- Tells the auto-tagger (ORG-06) when the tag applies.
    description TEXT NOT NULL DEFAULT '',
    auto_apply INTEGER NOT NULL DEFAULT 0 CHECK (auto_apply IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE meeting_tags (
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    applied_by TEXT NOT NULL CHECK (applied_by IN ('user', 'ai', 'automation')),
    applied_at TEXT NOT NULL,
    PRIMARY KEY (meeting_id, tag_id)
) STRICT;

CREATE INDEX idx_meeting_tags_tag_id ON meeting_tags (tag_id, meeting_id);

-- Action items ---------------------------------------------------------------

CREATE TABLE action_items (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- NULL for standalone items. Deleting a meeting keeps its items, since
    -- the user may have edited or completed them.
    meeting_id TEXT REFERENCES meetings (id) ON DELETE SET NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    assignee_person_id TEXT REFERENCES people (id) ON DELETE SET NULL,
    due_at TEXT,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'done')),
    priority TEXT CHECK (priority IN ('low', 'medium', 'high')),
    -- The transcript moment the item was extracted from.
    source_segment_id TEXT REFERENCES transcript_segments (id) ON DELETE SET NULL,
    -- Links to synced copies in task tools; INT-03 defines the payload and its Rust type.
    external_refs_json TEXT CHECK (external_refs_json IS NULL OR json_valid(external_refs_json)),
    completed_at TEXT,
    created_by TEXT NOT NULL CHECK (created_by IN ('ai', 'user')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((status = 'done') = (completed_at IS NOT NULL))
) STRICT;

CREATE INDEX idx_action_items_meeting_id ON action_items (meeting_id, created_at, id);
CREATE INDEX idx_action_items_assignee_status_due_at ON action_items (assignee_person_id, status, due_at, id);
CREATE INDEX idx_action_items_source_segment_id ON action_items (source_segment_id);
-- The global list: open items by due date.
CREATE INDEX idx_action_items_status_due_at ON action_items (status, due_at, id);
