-- TRN-04 clips and TRN-06 comments (ADR 0048).
--
-- A clip is a named range of a meeting's recording, cut into its own file
-- under the meeting's recording folder (`<recording dir>/clips/<name>.m4a`,
-- or `.mp4` when the meeting has a screen recording). Comments are the
-- user's private annotations: a thread anchored to a span of one transcript
-- line or of the notes, and its comments in order.

CREATE TABLE clips (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    -- Milliseconds from the start of the recording, end exclusive.
    start_ms INTEGER NOT NULL CHECK (start_ms >= 0),
    end_ms INTEGER NOT NULL CHECK (end_ms > start_ms),
    -- `audio` (an .m4a of the mixed recording) or `video` (an .mp4 of the
    -- screen recording).
    media TEXT NOT NULL CHECK (media IN ('audio', 'video')),
    -- The clip's file, absolute like `meetings.audio_path`; one row per file.
    path TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- The meeting's Clips tab lists its clips in time order (and the FK).
CREATE INDEX idx_clips_meeting_id_start_ms ON clips (meeting_id, start_ms, id);
-- The ⌘K palette lists every clip, newest first.
CREATE INDEX idx_clips_created_at ON clips (created_at, id);

CREATE TABLE comment_threads (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    -- What the thread annotates: a transcript line, "My notes" or the
    -- enhanced notes.
    target TEXT NOT NULL CHECK (target IN ('transcript', 'notes', 'enhanced')),
    -- The transcript line (target `transcript`). A re-transcription that
    -- deletes the line leaves the thread detached rather than losing it.
    segment_id TEXT REFERENCES transcript_segments (id) ON DELETE SET NULL,
    -- Character offsets (UTF-16 code units, as the webview counts) of the
    -- span in the line's text or the notes' text, end exclusive.
    start_offset INTEGER NOT NULL CHECK (start_offset >= 0),
    end_offset INTEGER NOT NULL CHECK (end_offset > start_offset),
    -- The span's text when commented: finds the span again after an edit
    -- moves it, and tells when the edit removed it ("detached").
    quote TEXT NOT NULL CHECK (length(quote) > 0),
    resolved_at TEXT,
    created_at TEXT NOT NULL,
    CHECK (target = 'transcript' OR segment_id IS NULL)
) STRICT;

-- A meeting's threads in creation order (and the FK).
CREATE INDEX idx_comment_threads_meeting_id ON comment_threads (meeting_id, created_at, id);
CREATE INDEX idx_comment_threads_segment_id ON comment_threads (segment_id);

CREATE TABLE comments (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    thread_id TEXT NOT NULL REFERENCES comment_threads (id) ON DELETE CASCADE,
    body TEXT NOT NULL CHECK (length(trim(body)) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- A thread's comments in order (and the FK).
CREATE INDEX idx_comments_thread_id ON comments (thread_id, created_at, id);
