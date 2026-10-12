-- Finished meetings outside the trash saved at or before ?1: what "everything"
-- moves to the trash, oldest first. A meeting still scheduled, recording or
-- processing is never touched.
SELECT m.id AS "id!: MeetingId"
FROM meetings AS m
WHERE m.deleted_at IS NULL
    AND m.status IN ('ready', 'failed')
    AND julianday(m.created_at) <= julianday(?1)
ORDER BY m.created_at, m.id
