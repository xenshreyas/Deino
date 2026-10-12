-- PRV-01: retention policies (ADR 2026-10-12-retention-runs-with-the-trash-purge).
--
-- Settings → Privacy & security keeps recordings, transcripts or whole
-- meetings for a chosen number of days. The retention sweep, which runs on
-- the trash purge's schedule, deletes what has expired:
--
-- - recordings: the files under `recordings/<id>` (clips included), the
--   meeting's `audio_*` paths, its merge parts' paths and its `clips` rows;
-- - transcripts: `transcript_segments` (the FTS triggers drop their search
--   documents) and the transcript's `embeddings`; notes stay;
-- - everything: the meeting moves to the trash, and the 30-day purge
--   deletes it for good.
--
-- Two stamps record what retention removed, so the meeting page can say
-- "Transcript deleted" and refuse to regenerate notes. Both stay NULL for a
-- meeting retention never touched. The sweep finds candidates through
-- idx_meetings_live_created_at_id (meetings outside the trash, by age).

-- When retention deleted the meeting's recording files.
ALTER TABLE meetings ADD COLUMN audio_deleted_at TEXT;
-- When retention deleted the meeting's transcript. The notes stay, but
-- cannot be regenerated without a transcript.
ALTER TABLE meetings ADD COLUMN transcript_deleted_at TEXT;
