-- What the summary of a person (?1) reads: their latest ?2 meetings outside
-- the trash that already happened, most recent first, each with its title,
-- its finished summary and the user's own notes. The summary is the legacy
-- `summary_processes.result` Markdown until NOT-02's enhanced notes replace
-- it (the same read as the auto-tagger's digest); a malformed result reads
-- as no summary.
SELECT
    m.id AS "meeting_id!: MeetingId",
    m.title,
    coalesce(m.started_at, m.created_at) AS "at!: DateTime<Utc>",
    (
        SELECT CASE WHEN json_valid(sp.result) THEN json_extract(sp.result, '$.markdown') END
        FROM summary_processes AS sp
        WHERE sp.meeting_id = m.id AND lower(sp.status) = 'completed'
    ) AS "summary?: String",
    (SELECT n.notes_markdown FROM meeting_notes AS n WHERE n.meeting_id = m.id) AS "notes?: String"
FROM meeting_attendees AS a
JOIN meetings AS m ON m.id = a.meeting_id
WHERE a.person_id = ?1 AND m.deleted_at IS NULL AND m.status <> 'scheduled'
ORDER BY 3 DESC, m.id DESC
LIMIT ?2
