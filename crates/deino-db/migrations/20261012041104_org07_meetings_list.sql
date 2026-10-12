-- ORG-07: the Meetings page's saved views, and the parts of merged meetings.
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, timestamps RFC 3339
-- UTC, explicit ON DELETE, an index for every list order.
--
-- The page's filters need no new index: the list walks
-- idx_meetings_live_created_at_id newest first and probes each candidate
-- through the primary keys of meeting_attendees, meeting_tags and
-- meeting_folders and idx_action_items_meeting_id (asserted with EXPLAIN
-- QUERY PLAN in `meetings/list/repo_tests.rs`). The 30-day purge reads
-- idx_meetings_trashed_deleted_at.

-- Saved views ----------------------------------------------------------------
-- A view is a named filter the page shows as a tab. The filter is a typed
-- Rust value (`deino_core::meetings::MeetingFilter`) stored as JSON: the page
-- reads a view whole and the list query takes the parsed filter, so no SQL
-- ever reads a field of it (PARITY_PLAN §3A "JSON columns"; the same shape
-- as AI-05's action_item_views).

CREATE TABLE meeting_views (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    name TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK (length(trim(name)) > 0),
    filter_json TEXT NOT NULL
        CHECK (json_valid(filter_json) AND json_type(filter_json) = 'object'),
    -- 0-based order of the tabs; `SqliteMeetingRepo::create_view` appends.
    position INTEGER NOT NULL CHECK (position >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- The tabs, in order.
CREATE INDEX idx_meeting_views_position ON meeting_views (position, id);

-- Merged meetings ------------------------------------------------------------
-- Merging moves every row of the later meetings (transcript, speakers,
-- attendees, tags, folders, action items, notes) into the earliest one and
-- deletes them. Each absorbed meeting leaves one part here, so its
-- recording stays reachable: the player plays part N's files from
-- `offset_ms` on the merged timeline, and deleting the merged meeting for
-- good also deletes each part's files and folders. The merged meeting's own
-- recording is part 0 at offset 0 and has no row.
--
-- `source_meeting_id` has no foreign key: the absorbed row is gone, and the
-- ID only names its folders (`recordings/<id>`, `meetings/<id>`), which keep
-- its recording and note images.
--
-- The merge also rewrites `meetings.duration_s` (denormalised, FND-07) of the
-- merged meeting to the length of the whole timeline, in the same
-- transaction (`SqliteMeetingRepo::apply_merge`).

CREATE TABLE meeting_merge_parts (
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    -- 1-based order on the merged timeline.
    position INTEGER NOT NULL CHECK (position >= 1),
    source_meeting_id TEXT NOT NULL CHECK (length(trim(source_meeting_id)) > 0),
    title TEXT NOT NULL,
    -- When the absorbed meeting took place (its start, else when it was saved).
    held_at TEXT NOT NULL,
    -- Where the part starts on the merged timeline, and how long it is.
    offset_ms INTEGER NOT NULL CHECK (offset_ms >= 0),
    duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
    -- The absorbed meeting's recording files (`meetings.audio_*` as they were).
    audio_path TEXT,
    audio_mic_path TEXT,
    audio_system_path TEXT,
    video_path TEXT,
    merged_at TEXT NOT NULL,
    PRIMARY KEY (meeting_id, position)
) STRICT;
