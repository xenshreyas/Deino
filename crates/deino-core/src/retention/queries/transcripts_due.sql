-- Finished meetings outside the trash saved at or before ?1 whose transcript is
-- still kept and has a line: the transcripts retention deletes, oldest first.
SELECT m.id AS "id!: MeetingId"
FROM meetings AS m
WHERE m.deleted_at IS NULL
    AND m.transcript_deleted_at IS NULL
    AND m.status IN ('ready', 'failed')
    AND julianday(m.created_at) <= julianday(?1)
    AND EXISTS (SELECT 1 FROM transcript_segments AS s WHERE s.meeting_id = m.id)
ORDER BY m.created_at, m.id
