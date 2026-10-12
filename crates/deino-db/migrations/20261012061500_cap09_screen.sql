-- CAP-09: screen capture while a meeting records.
--
-- The video itself is a file (`<recordings>/<meeting>/video.mp4`, named by
-- `meetings.video_path`); these tables hold the snapshots and the user's
-- per-meeting choice.

-- `snapshots`: pictures of the meeting window kept while it records, every
-- 10 s at most and only when the screen changed (a perceptual hash more
-- than 10 of 64 bits from the last kept one). The PNG lives in the
-- meeting's recording folder, `snapshots/<file_name>`, and goes with it
-- (ORG-07's purge deletes the folder; the rows cascade). `at_ms` is the
-- offset on the recording's own timeline (pauses skipped), like
-- `transcript_segments`. `phash` is the 64-bit hash's bits as a signed
-- integer, kept so later snapshots and the `screen_context` stage can
-- compare pictures without reading them. Rows are appended by the capture
-- (`ScreenRepo::add_snapshot`) and deleted only by the user.
CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    at_ms INTEGER NOT NULL CHECK (at_ms >= 0),
    file_name TEXT NOT NULL CHECK (file_name GLOB '[0-9]*.png'),
    width INTEGER NOT NULL CHECK (width > 0),
    height INTEGER NOT NULL CHECK (height > 0),
    phash INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (meeting_id, file_name)
) STRICT;

-- The FK, and the meeting page's strip in time order.
CREATE INDEX idx_snapshots_meeting_id ON snapshots (meeting_id, at_ms);

-- `screen_capture_choices`: "Turn off for this meeting". A row with
-- `capture = 0` keeps screen capture off for that meeting, whatever the
-- global setting says (a resumed recording does not restart it); no row
-- means the setting decides. Written only by `ScreenRepo::set_capture`.
CREATE TABLE screen_capture_choices (
    meeting_id TEXT PRIMARY KEY NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    -- 1 captures, 0 never does.
    capture INTEGER NOT NULL CHECK (capture IN (0, 1)),
    updated_at TEXT NOT NULL
) STRICT;
