-- What meeting ?1 is related by: its title, series, when it took place and
-- its attendees (space-separated IDs, in no particular order).
SELECT
    m.id AS "id!: MeetingId",
    m.title AS "title!",
    m.series_id AS "series_id: SeriesId",
    COALESCE(m.started_at, m.created_at) AS "happened_at!: DateTime<Utc>",
    (SELECT group_concat(a.person_id, ' ') FROM meeting_attendees a WHERE a.meeting_id = m.id)
        AS "attendees: String"
FROM meetings m
WHERE m.id = ?1
