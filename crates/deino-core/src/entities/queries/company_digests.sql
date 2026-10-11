-- What the summary of a company (?1) reads: the latest ?2 meetings outside
-- the trash with any of its listed people, most recent first, with titles,
-- finished summaries and the user's notes (see person_digests.sql).
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
FROM meetings AS m
WHERE m.deleted_at IS NULL AND m.status <> 'scheduled' AND m.id IN (
    SELECT a.meeting_id
    FROM people AS p
    JOIN meeting_attendees AS a ON a.person_id = p.id
    WHERE p.company_id = ?1 AND p.merged_into IS NULL AND p.is_me = 0
)
ORDER BY 3 DESC, m.id DESC
LIMIT ?2
