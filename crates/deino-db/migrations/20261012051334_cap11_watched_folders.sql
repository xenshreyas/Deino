-- CAP-11: folders Deino watches for recordings to import, and the files they
-- handled.
--
-- `watched_folders`: Settings → Watched folders. New audio and video files in
-- `path` (not in its subfolders) that have stopped changing for 10 s are
-- imported; with `move_to_processed` the file then moves into the folder's
-- `Processed/` subfolder.
CREATE TABLE watched_folders (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    path TEXT NOT NULL UNIQUE CHECK (length(path) > 0),
    move_to_processed INTEGER NOT NULL DEFAULT 0 CHECK (move_to_processed IN (0, 1)),
    created_at TEXT NOT NULL
) STRICT;

-- `watched_files`: one row per file a watched folder handled, written by
-- `WatchService` once the import succeeded or failed. It is the
-- de-duplication ledger, so a file is imported at most once: the same path,
-- size and modification time is never picked up again, across restarts, even
-- after its meeting is deleted. Files already in a folder when it is added
-- are entered with neither a meeting nor a failure, so only new ones import.
-- Rows with a `failure` and no `dismissed_at` are the failure list; retrying
-- deletes the row, so the next scan imports the file again, and dismissing
-- stamps `dismissed_at`, so it never does.
CREATE TABLE watched_files (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    folder_id TEXT NOT NULL REFERENCES watched_folders (id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    modified_ms INTEGER NOT NULL,
    meeting_id TEXT REFERENCES meetings (id) ON DELETE SET NULL,
    failure TEXT CHECK (failure IN ('missing', 'too_large', 'no_audio', 'unsupported', 'damaged', 'failed')),
    handled_at TEXT NOT NULL,
    dismissed_at TEXT
) STRICT;

-- The ledger lookup; also serves `folder_id`'s foreign key.
CREATE UNIQUE INDEX idx_watched_files_fingerprint ON watched_files (folder_id, path, size_bytes, modified_ms);
CREATE INDEX idx_watched_files_meeting_id ON watched_files (meeting_id);
-- The failure list, newest first.
CREATE INDEX idx_watched_files_failures ON watched_files (handled_at)
    WHERE failure IS NOT NULL AND dismissed_at IS NULL;
