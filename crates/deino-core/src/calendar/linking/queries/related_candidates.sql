-- Meetings outside the trash, other than ?1, in series ?2 or titled exactly
-- ?3, newest first, at most ?4. Each side of the union searches its index
-- (idx_meetings_series_id, idx_meetings_title); an OR would scan one.
WITH matches (id) AS (
    SELECT id FROM meetings WHERE series_id = ?2
    UNION
    SELECT id FROM meetings WHERE title = ?3 AND deleted_at IS NULL
)
SELECT
    m.id AS "id!: MeetingId",
    m.title AS "title!",
    m.series_id AS "series_id: SeriesId",
    COALESCE(m.started_at, m.created_at) AS "happened_at!: DateTime<Utc>",
    (SELECT group_concat(a.person_id, ' ') FROM meeting_attendees a WHERE a.meeting_id = m.id)
        AS "attendees: String"
FROM matches
JOIN meetings m ON m.id = matches.id
WHERE m.deleted_at IS NULL
  AND m.id <> ?1
ORDER BY COALESCE(m.started_at, m.created_at) DESC, m.id
LIMIT ?4
