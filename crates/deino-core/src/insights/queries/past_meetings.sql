-- Finished meetings outside the trash that have a transcript, newest first:
-- what "Run on past meetings" offers.
SELECT
    m.id AS "id!: MeetingId",
    m.title,
    coalesce(m.started_at, m.created_at) AS "started_at!: DateTime<Utc>"
FROM meetings AS m
WHERE m.deleted_at IS NULL AND m.status IN ('ready', 'failed')
    AND EXISTS (
        SELECT 1 FROM transcript_segments AS s
        WHERE s.meeting_id = m.id AND s.is_deleted = 0
    )
ORDER BY coalesce(m.started_at, m.created_at) DESC, m.id
