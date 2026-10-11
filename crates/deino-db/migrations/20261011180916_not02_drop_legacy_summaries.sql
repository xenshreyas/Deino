-- NOT-02: the legacy summaries are gone; enhanced notes replace them.
--
-- Before this migration runs, the Rust data migration
-- `deino_core::enhance::LegacySummaryImport` (a `deino_db::DataMigration`
-- whose `runs_before` is this file's version) copies every completed legacy
-- summary into `enhanced_notes` as version 1, its text marked as written by
-- the AI. `transcript_chunks` only held the transcript text sent to the
-- legacy summariser (and a copy of the meeting's title); the transcript
-- itself lives in `transcript_segments`.

DROP TABLE IF EXISTS summary_processes;
DROP TABLE IF EXISTS transcript_chunks;
