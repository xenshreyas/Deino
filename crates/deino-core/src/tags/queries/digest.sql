-- A meeting's (?1) title and finished summary, for the auto-tagger; no row
-- when the meeting is missing or in the trash. The summary is the legacy
-- `summary_processes.result` Markdown until NOT-02's enhanced notes replace
-- it; a malformed result reads as no summary.
SELECT
    m.id AS "meeting_id!: MeetingId",
    m.title,
    (
        SELECT CASE WHEN json_valid(sp.result) THEN json_extract(sp.result, '$.markdown') END
        FROM summary_processes AS sp
        WHERE sp.meeting_id = m.id AND lower(sp.status) = 'completed'
    ) AS "summary?: String"
FROM meetings AS m
WHERE m.id = ?1 AND m.deleted_at IS NULL
