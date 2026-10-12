-- TRN-02: transcript edits (inline edit, delete, find and replace).
--
-- 1. `original_text` keeps what the engine heard, so an edit can be undone
--    and compared. NULL while the segment was never edited; set once, by the
--    first edit, to the text before it, and never changed after. The one
--    writer is `SqliteSegmentRepo` (`edit_text`, `edit_many`), in the same
--    statement that sets `text` and `edited_at`.
-- 2. The semantic index (`embeddings`, ADR 0032) copies transcript windows'
--    text. A window covering a segment the user edits or deletes is stale the
--    moment the edit commits, and deleted words must never be found again:
--    these triggers drop such windows in the edit's own transaction. The
--    transcript's re-index (`search::ReindexQueue`) embeds them again from
--    the current text. Keyword search (`search_documents`) already follows
--    `text` and `is_deleted` through its ORG-08 triggers.
-- 3. The legacy `transcripts` view hides soft-deleted segments, so a legacy
--    `DELETE FROM transcripts WHERE meeting_id = ?` (re-transcription, ADR
--    0012) would leave them behind and an undo could bring back lines of the
--    old transcription. Deleting through the view now also removes the
--    meeting's soft-deleted segments.

ALTER TABLE transcript_segments ADD COLUMN original_text TEXT;

-- A user edit (the text changes with a new `edited_at`) or a soft delete.
-- Diarization rewrites `text` without `edited_at` (CAP-03 splits segments
-- at speaker changes); its words were said, so its windows stay until the
-- `index` stage re-embeds the meeting.
CREATE TRIGGER embeddings_segments_after_edit
AFTER UPDATE OF text, is_deleted, edited_at ON transcript_segments
WHEN (NEW.is_deleted = 1 AND OLD.is_deleted = 0) OR NEW.edited_at IS NOT OLD.edited_at
BEGIN
    DELETE FROM embeddings
    WHERE meeting_id = OLD.meeting_id
        AND ref_kind = 'transcript'
        AND ts_ms <= OLD.end_ms
        AND end_ms >= OLD.start_ms;
END;

DROP TRIGGER transcripts_delete;

CREATE TRIGGER transcripts_delete
INSTEAD OF DELETE ON transcripts
BEGIN
    DELETE FROM transcript_segments
    WHERE id = OLD.id OR (meeting_id = OLD.meeting_id AND is_deleted = 1);
END;
