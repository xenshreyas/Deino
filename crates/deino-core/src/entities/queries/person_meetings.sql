-- The meetings outside the trash that a person (?1) attended and that
-- already happened, most recent first, at most ?2. Filtered through
-- idx_meeting_attendees_person_id and joined on the meetings primary key;
-- one person's meetings are few enough to sort.
SELECT
    m.id AS "id!: MeetingId",
    m.title,
    coalesce(m.started_at, m.created_at) AS "at!: DateTime<Utc>",
    m.duration_s AS "duration_s: u32"
FROM meeting_attendees AS a
JOIN meetings AS m ON m.id = a.meeting_id
WHERE a.person_id = ?1 AND m.deleted_at IS NULL AND m.status <> 'scheduled'
ORDER BY 3 DESC, m.id DESC
LIMIT ?2
