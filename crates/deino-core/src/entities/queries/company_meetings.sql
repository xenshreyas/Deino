-- The meetings outside the trash that any listed person at a company (?1)
-- attended and that already happened, each once, most recent first, at most
-- ?2. The company's people come through idx_people_company_id and their
-- attendance through idx_meeting_attendees_person_id.
SELECT
    m.id AS "id!: MeetingId",
    m.title,
    coalesce(m.started_at, m.created_at) AS "at!: DateTime<Utc>",
    m.duration_s AS "duration_s: u32"
FROM meetings AS m
WHERE m.deleted_at IS NULL AND m.status <> 'scheduled' AND m.id IN (
    SELECT a.meeting_id
    FROM people AS p
    JOIN meeting_attendees AS a ON a.person_id = p.id
    WHERE p.company_id = ?1 AND p.merged_into IS NULL AND p.is_me = 0
)
ORDER BY 3 DESC, m.id DESC
LIMIT ?2
